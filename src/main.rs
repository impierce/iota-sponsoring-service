use composition_root::CompositionRoot;
use std::env;
use tracing::{info, instrument};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
#[instrument(name = "app_startup")]
async fn main() {
    dotenvy::dotenv().ok();

    let log_format = env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());

    // Initialize the subscriber
    let subscriber = FmtSubscriber::builder().with_env_filter(EnvFilter::from_default_env());
    if log_format == "json" {
        let subscriber = subscriber
            .json() // <--- This is the key change
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    } else {
        let subscriber = subscriber.finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    };

    // Read address from environment variable with fallback
    let addr = env::var("SERVER_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8000".to_string());

    // Read MongoDB URI from environment variable with fallback
    let mongo_uri = env::var("MONGODB_URI")
        .unwrap_or_else(|_| "mongodb://localhost:27017/iota-sponsoring-service?directConnection=true&retryWrites=false&replicaSet=rs0".to_string());

    // Read Gas Station config path from environment variable with fallback
    let gas_station_config_path = env::var("GAS_STATION_CONFIG_PATH")
        .unwrap_or_else(|_| "./gas-station.config.yaml".to_string());

    // Check whether CORS needs to be enabled
    let cors_enabled = env::var("CORS_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        == "true";

    // Check whether Demo data initialization is enabled
    let demo_data_initialization_enabled = env::var("DEMO_DATA_INITIALIZATION_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        == "true";

    info!(log_format = %log_format, "Log format configured");
    info!(server_address = %addr, "Server address configured");
    info!(mongodb_uri = %mongo_uri, "MongoDB URI configured");
    info!(gas_station_config_path = %gas_station_config_path, "Gas station config path configured");

    info!("Starting server");

    let composition_root = CompositionRoot::new(
        mongo_uri,
        gas_station_config_path,
        demo_data_initialization_enabled,
    )
    .await;

    axum_graphql::run(&addr, cors_enabled, composition_root).await;
}
