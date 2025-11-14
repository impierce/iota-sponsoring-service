pub mod graphql_endpoint_service;
pub mod operations;

use std::sync::Arc;

use application::{
    services::{
        allocation_service::AllocationService,
        authorize_transaction_service::AuthorizeTransactionService,
    },
    views::sponsor_wallet::SPONSOR_WALLET_VIEW_ID,
};
use async_graphql::{Schema, http::GraphiQLSource};
use axum::{
    Json, Router,
    extract::State,
    response::{self, IntoResponse},
    routing::{get, post},
};
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use composition_root::CompositionRoot;
use cqrs_es::persist::PersistedEventStore;
use iota_gas_station::access_controller::hook::{
    ExecuteTxHookRequest, ExecuteTxOkResponse, SkippableDecision,
};
use mongo_es::MongoEventRepository;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};
use wallet_integration::sponsor_wallet::aggregate::SponsorWallet;

use crate::{
    graphql_endpoint_service::GraphQLEndpointService,
    operations::{mutations::MutationRoot, queries::QueryRoot, subscriptions::SubscriptionRoot},
};

async fn graphiql() -> impl IntoResponse {
    response::Html(
        GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/graphql")
            .finish(),
    )
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
    pub message: String,
}

async fn read_log_events(
    events: Vec<Event>,
    allocation_service: Arc<
        AllocationService<
            PersistedEventStore<MongoEventRepository, SponsorWallet>,
            PersistedEventStore<MongoEventRepository, Client>,
            PersistedEventStore<MongoEventRepository, Group>,
        >,
    >,
) {
    let prior_balance_regex = regex::Regex::new(
        r"Total gas coin balance prior to execution: (?P<balance>\d+) reservation_id=",
    )
    .unwrap();
    let executing_transaction_regex =
        regex::Regex::new(r"Executing transaction:.*?sender: (?P<sender>0x[0-9a-fA-F]+)").unwrap();
    let after_balance_regex = regex::Regex::new(
        r"New gas coin balance after execution: (?P<balance>\d+) reservation_id=",
    )
    .unwrap();
    let new_total_balance_regex =
        regex::Regex::new(r"After add_new_coins. New total balance: (?P<balance>\d+)").unwrap();

    let mut prior = 0;
    let mut after = 0;
    let mut wallet_addresss = String::new();
    let mut new_total_balance = 0;

    for event in events {
        if let Some(captures) = prior_balance_regex.captures(&event.message) {
            if let Some(balance_match) = captures.name("balance") {
                match balance_match.as_str().parse::<u64>() {
                    Ok(balance_prior) => {
                        info!("Successfully parsed prior balance: {balance_prior}");

                        prior = balance_prior;
                    }
                    Err(e) => {
                        warn!("Failed to parse balance from log: {}", e);
                    }
                }
            }
        } else if let Some(captures) = executing_transaction_regex.captures(&event.message) {
            if let Some(sender_match) = captures.name("sender") {
                info!("Successfully parsed sender: {}", sender_match.as_str());

                wallet_addresss = sender_match.as_str().to_string();
            }
        } else if let Some(captures) = after_balance_regex.captures(&event.message) {
            if let Some(balance_match) = captures.name("balance") {
                match balance_match.as_str().parse::<u64>() {
                    Ok(balance_after) => {
                        info!("Successfully parsed new balance: {balance_after}");

                        after = balance_after;
                    }
                    Err(e) => {
                        warn!("Failed to parse balance from log: {}", e);
                    }
                }
            }
        } else if let Some(captures) = new_total_balance_regex.captures(&event.message) {
            if let Some(balance_match) = captures.name("balance") {
                match balance_match.as_str().parse::<u64>() {
                    Ok(new_total_balance_match) => {
                        info!("Successfully parsed new total balance: {new_total_balance_match}");

                        new_total_balance = new_total_balance_match;

                        allocation_service
                            .record_balance_update(
                                SPONSOR_WALLET_VIEW_ID.to_string(),
                                new_total_balance,
                            )
                            .await
                            .unwrap();
                    }
                    Err(e) => {
                        warn!("Failed to parse new total balance from log: {}", e);
                    }
                }
            }
        } else {
            // warn!("Could not find JSON in log message: {}", event.message);
        }
    }

    if prior > 0 {
        let transaction_fee = prior - after;

        allocation_service
            .record_transaction_fee_paid(wallet_addresss.clone(), transaction_fee)
            .await
            .unwrap();
    }
}

async fn handle_transaction_webhook(
    State(allocation_service): State<
        Arc<
            AllocationService<
                PersistedEventStore<MongoEventRepository, SponsorWallet>,
                PersistedEventStore<MongoEventRepository, Client>,
                PersistedEventStore<MongoEventRepository, Group>,
            >,
        >,
    >,
    Json(events): Json<Vec<Event>>,
) {
    read_log_events(events, allocation_service).await;
}

async fn authorize_transaction_webhook(
    State(authorize_transaction_service): State<
        Arc<
            AuthorizeTransactionService<
                PersistedEventStore<MongoEventRepository, SponsorWallet>,
                PersistedEventStore<MongoEventRepository, Client>,
                PersistedEventStore<MongoEventRepository, Group>,
            >,
        >,
    >,
    Json(transaction_data): Json<ExecuteTxHookRequest>,
    // FIXME: proper error handling
) -> Result<Json<ExecuteTxOkResponse>, String> {
    match authorize_transaction_service
        .authorize_transaction(transaction_data)
        .await
    {
        Ok(_) => Ok(Json(ExecuteTxOkResponse {
            decision: SkippableDecision::Allow,
            user_message: None,
        })),
        Err(e) => {
            info!("Failed to authorize transaction: {}", e);
            Err(format!("Failed to authorize transaction: {}", e))
        }
    }
}

async fn app(
    cors_enabled: bool,
    CompositionRoot {
        allocation_service,
        authorize_transaction_service,
        balance_management_service,
        sponsor_wallet_view,
        client_view,
        client_list_view,
        sponsorship_transaction_list_view,
        group_view,
        group_list_view,
        sponsor_wallet_query_receiver,
        client_query_receiver,
        group_query_receiver,
    }: CompositionRoot,
) -> Router {
    let query_root = QueryRoot::new(
        sponsor_wallet_view,
        client_view,
        client_list_view,
        sponsorship_transaction_list_view.clone(),
        group_view,
        group_list_view,
    );
    let mutation_root = MutationRoot::new(allocation_service.clone(), balance_management_service);
    let subscription_root = SubscriptionRoot::new(
        sponsor_wallet_query_receiver,
        client_query_receiver,
        group_query_receiver,
        sponsorship_transaction_list_view,
    );
    let schema = Schema::new(query_root, mutation_root, subscription_root);

    let app = Router::new()
        .route("/", get(graphiql))
        .route("/webhook/transaction", post(handle_transaction_webhook))
        .with_state(allocation_service)
        .route(
            "/webhook/authorize-transaction",
            post(authorize_transaction_webhook),
        )
        .with_state(authorize_transaction_service)
        .route_service("/graphql", GraphQLEndpointService::new(schema));

    if cors_enabled {
        app.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
    } else {
        app
    }
}

pub async fn run(addr: &str, cors_enabled: bool, composition_root: CompositionRoot) {
    let app = app(cors_enabled, composition_root).await;

    info!("GraphiQL IDE: http://{}", addr);
    info!("GraphQL endpoint: http://{}/graphql", addr);

    axum::serve(TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}
