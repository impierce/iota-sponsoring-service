pub mod graphql_endpoint_service;
pub mod operations;

use async_graphql::{Schema, http::GraphiQLSource};
use axum::{
    Router,
    response::{self, IntoResponse},
    routing::get,
};
use composition_root::CompositionRoot;
use tokio::net::TcpListener;

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

async fn app(
    CompositionRoot {
        balance_management_service,
        client_view,
        client_list_view,
        group_view,
        group_list_view,
        client_query_receiver,
        group_query_receiver,
    }: CompositionRoot,
) -> Router {
    let query_root = QueryRoot::new(client_view, client_list_view, group_view, group_list_view);
    let mutation_root = MutationRoot::new(balance_management_service);
    let subscription_root = SubscriptionRoot::new(client_query_receiver, group_query_receiver);
    let schema = Schema::new(query_root, mutation_root, subscription_root);

    Router::new()
        .route("/", get(graphiql))
        .route_service("/graphql", GraphQLEndpointService::new(schema))
}

pub async fn run(addr: &str, composition_root: CompositionRoot) {
    let app = app(composition_root).await;

    println!("GraphiQL IDE: http://{}", addr);
    println!("GraphQL endpoint: http://{}/graphql", addr);

    axum::serve(TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}
