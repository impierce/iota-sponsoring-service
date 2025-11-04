use composition_root::CompositionRoot;
use std::env;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Read address from environment variable with fallback
    let addr = env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8000".to_string());

    // Read MongoDB URI from environment variable with fallback
    let mongo_uri = env::var("MONGODB_URI")
        .unwrap_or_else(|_| "mongodb://localhost:27017/ssi-agent?directConnection=true&retryWrites=false&replicaSet=rs0".to_string());

    // Read Gas Station config path from environment variable with fallback
    let gas_station_config_path = env::var("GAS_STATION_CONFIG_PATH")
        .unwrap_or_else(|_| "./gas-station.config.yaml".to_string());

    println!("Starting server on: {}", addr);

    let composition_root = CompositionRoot::new(mongo_uri, gas_station_config_path).await;

    axum_graphql::run(&addr, composition_root).await;
}
