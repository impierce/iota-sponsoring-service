use async_graphql::{EmptyMutation, EmptySubscription, Schema, http::GraphiQLSource};
use async_graphql_axum::GraphQL;
use axum::{
    Router,
    response::{self, IntoResponse},
    routing::get,
};
use tokio::net::TcpListener;

use crate::model::Query;

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").finish())
}

fn app() -> Router {
    let schema = Schema::new(Query, EmptyMutation, EmptySubscription);

    Router::new().route("/graphql", get(graphiql).post_service(GraphQL::new(schema)))
}

pub async fn run(addr: &str) {
    let app = app();

    axum::serve(TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

pub mod model {

    use async_graphql::Object;

    pub struct Query;

    #[Object]
    impl Query {
        /// Returns the token balance (mocked value)
        async fn token_balance(&self) -> i32 {
            12345
        }
    }
}
