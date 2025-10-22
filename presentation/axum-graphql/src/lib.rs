use async_graphql::{EmptyMutation, Schema, http::GraphiQLSource};
use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::{
    Router,
    response::{self, IntoResponse},
    routing::get,
};
use tokio::net::TcpListener;

use crate::model::{Query, TokenSubscription};

async fn graphiql() -> impl IntoResponse {
    response::Html(
        GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/graphql/ws")
            .finish(),
    )
}

fn app() -> Router {
    let schema = Schema::new(Query::new(), EmptyMutation, TokenSubscription::new());

    Router::new()
        .route("/", get(graphiql))
        .route(
            "/graphql",
            get(|| async { "GraphQL endpoint - use POST for queries" })
                .post_service(GraphQL::new(schema.clone())),
        )
        .route_service("/graphql/ws", GraphQLSubscription::new(schema))
}

pub async fn run(addr: &str) {
    let app = app();

    println!("GraphiQL IDE: http://{}", addr);
    println!("GraphQL endpoint: http://{}/graphql", addr);
    println!("WebSocket endpoint: ws://{}/graphql/ws", addr);

    axum::serve(TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

pub mod model {

    use async_graphql::{Object, SimpleObject, Subscription};
    use futures_util::{Stream, StreamExt};
    use rand::Rng;
    use std::sync::Arc;
    use tokio::sync::{RwLock, broadcast};
    use tokio::time::{Duration, Instant};

    #[derive(Clone, SimpleObject)]
    pub struct TokenBalanceUpdate {
        pub balance: i32,
        pub timestamp: String,
        pub action: String, // "decreased" or "reset"
    }

    #[derive(Clone)]
    pub struct TokenBalance {
        value: i32,
        last_updated: Instant,
        sender: broadcast::Sender<TokenBalanceUpdate>,
    }

    impl TokenBalance {
        fn new() -> (Self, broadcast::Receiver<TokenBalanceUpdate>) {
            let (sender, receiver) = broadcast::channel(100);
            let balance = Self {
                value: 50000,
                last_updated: Instant::now(),
                sender,
            };
            (balance, receiver)
        }

        fn update(&mut self) {
            let decrease = rand::rng().random_range(100..=2000);
            let (action, new_value) = if self.value <= decrease {
                ("reset".to_string(), 50000)
            } else {
                ("decreased".to_string(), self.value - decrease)
            };

            self.value = new_value;
            self.last_updated = Instant::now();

            let update = TokenBalanceUpdate {
                balance: self.value,
                timestamp: chrono::Utc::now().to_rfc3339(),
                action,
            };

            println!("Token balance {}: {}", update.action, self.value);
            let _ = self.sender.send(update);
        }
    }

    // Shared state using std::sync::OnceLock (no need for lazy_static)
    static SHARED_BALANCE: std::sync::OnceLock<Arc<RwLock<TokenBalance>>> =
        std::sync::OnceLock::new();

    fn get_shared_balance() -> &'static Arc<RwLock<TokenBalance>> {
        SHARED_BALANCE.get_or_init(|| {
            let (balance, _) = TokenBalance::new();
            let shared = Arc::new(RwLock::new(balance));

            // Start background task to update balance with random interval (2-5 seconds)
            let balance_clone = shared.clone();
            tokio::spawn(async move {
                loop {
                    // Random interval between 2-5 seconds
                    let random_seconds = rand::rng().random_range(2..=5);
                    tokio::time::sleep(Duration::from_secs(random_seconds)).await;

                    let mut balance = balance_clone.write().await;
                    balance.update();
                }
            });

            shared
        })
    }

    pub struct Query;

    impl Query {
        pub fn new() -> Self {
            // Initialize shared state
            let _ = get_shared_balance();
            Self
        }
    }

    #[Object]
    impl Query {
        /// Returns the token balance (decreases over time, resets to 50000 when it hits 0)
        async fn token_balance(&self) -> i32 {
            let balance = get_shared_balance().read().await;
            balance.value
        }
    }

    pub struct TokenSubscription {}

    impl TokenSubscription {
        pub fn new() -> Self {
            Self {}
        }
    }

    #[Subscription]
    impl TokenSubscription {
        /// Subscribe to token balance updates
        async fn token_balance_updates(&self) -> impl Stream<Item = TokenBalanceUpdate> {
            let balance = get_shared_balance().read().await;
            let receiver = balance.sender.subscribe();

            tokio_stream::wrappers::BroadcastStream::new(receiver)
                .filter_map(|result| async move { result.ok() })
        }
    }
}
