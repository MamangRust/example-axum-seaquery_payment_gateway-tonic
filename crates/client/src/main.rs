use anyhow::{Context, Result};
use dotenv::dotenv;
use seaquery_client_payment_gateway::{handler::AppRouter, state::AppState};
use shared::{
    config::Config,
    utils::{Telemetry, init_logger},
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let telemetry = Telemetry::new("myclient");

    let tracer_provider = telemetry.init_tracer();
    let meter_provider = telemetry.init_meter();
    let logger_provider = telemetry.init_logger();

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

    let mut shutdown_errors = Vec::new();

    if let Err(e) = tracer_provider.shutdown() {
        shutdown_errors.push(format!("tracer provider: {e}"));
    }
    if let Err(e) = meter_provider.shutdown() {
        shutdown_errors.push(format!("meter provider: {e}"));
    }
    if let Err(e) = logger_provider.shutdown() {
        shutdown_errors.push(format!("logger provider: {e}"));
    }

    if !shutdown_errors.is_empty() {
        anyhow::bail!(
            "Failed to shutdown providers:\n{}",
            shutdown_errors.join("\n")
        );
    }

    Ok(())
}
