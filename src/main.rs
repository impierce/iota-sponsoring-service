use std::env;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Read address from environment variable with fallback
    let addr = env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8000".to_string());

    println!("Starting server on: {}", addr);

    axum_graphql::run(&addr).await;
}
