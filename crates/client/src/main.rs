use anyhow::{Context, Result};
use dotenv::dotenv;
use seaquery_client_payment_gateway::{handler::AppRouter, state::AppState};
use shared::{
    config::Config,
    utils::{Telemetry, init_logger},
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let telemetry = Telemetry::new("myclient", "http://otel-collector:4317".to_string());

    let logger_provider = telemetry.init_logger();
    let _meter_provider = telemetry.init_meter();
    let _tracer_provider = telemetry.init_tracer();

    init_logger(logger_provider.clone(), "client");

    let config = Config::init().context("Failed to load configuration")?;

    let port = config.port;

    let state = AppState::new(&config.jwt_secret)
        .await
        .context("Failed to create AppState")?;

    println!("🚀 Server started successfully");

    AppRouter::serve(port, state)
        .await
        .context("Failed to start server")?;

    info!("Shutting down servers...");

    telemetry.shutdown().await?;

    Ok(())
}
