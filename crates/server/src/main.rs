use anyhow::{Context, Result};
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use genproto::{
    auth::auth_service_server::AuthServiceServer, saldo::saldo_service_server::SaldoServiceServer,
    topup::topup_service_server::TopupServiceServer,
    transfer::transfer_service_server::TransferServiceServer,
    user::user_service_server::UserServiceServer,
    withdraw::withdraw_service_server::WithdrawServiceServer,
};
use prometheus_client::encoding::text::encode;
use shared::{
    config::{Config, ConnectionManager},
    state::AppState,
    utils::{Telemetry, init_logger, shutdown_signal},
};
use std::sync::Arc;
use tracing::{error, info};

use crate::{config::ServerConfig, service::ServiceContainer};

mod config;
mod service;

pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut buffer = String::new();

    let registry = state.registry.lock().await;

    if let Err(e) = encode(&mut buffer, &registry) {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Failed to encode metrics: {e}")))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(Body::from(buffer))
        .unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let telemetry = Telemetry::new("myserver");
    let logger_provider = telemetry.init_logger();

    init_logger(logger_provider.clone(), "server");

    info!("Starting server initialization...");

    let config = Config::init().context("Failed to load configuration")?;
    let server_config = ServerConfig::from_config(&config)?;

    let db_pool =
        ConnectionManager::new_pool(&server_config.database_url, server_config.run_migrations)
            .await
            .context("Failed to initialize database pool")?;

    let state = Arc::new(
        AppState::new(db_pool, &server_config.jwt_secret)
            .await
            .context("Failed to create AppState")?,
    );

    let services = ServiceContainer::new(state.clone());

    let server_result = tokio::try_join!(
        start_grpc_server(services, server_config.grpc_addr),
        start_metrics_server(state, server_config.metrics_addr)
    );

    match server_result {
        Ok(_) => info!("Servers started successfully"),
        Err(e) => {
            error!("Server startup failed: {}", e);
            if let Err(shutdown_err) = telemetry.shutdown().await {
                error!("Failed to shutdown telemetry: {}", shutdown_err);
            }
            return Err(e);
        }
    }

    info!("Shutting down servers...");
    telemetry.shutdown().await?;

    Ok(())
}

async fn start_grpc_server(services: ServiceContainer, addr: std::net::SocketAddr) -> Result<()> {
    info!("Starting gRPC server on {}", addr);

    tonic::transport::Server::builder()
        .add_service(AuthServiceServer::new(services.auth))
        .add_service(UserServiceServer::new(services.user))
        .add_service(SaldoServiceServer::new(services.saldo))
        .add_service(TopupServiceServer::new(services.topup))
        .add_service(TransferServiceServer::new(services.transfer))
        .add_service(WithdrawServiceServer::new(services.withdraw))
        .serve_with_shutdown(addr, shutdown_signal())
        .await
        .with_context(|| format!("gRPC server failed on {addr}"))
}

async fn start_metrics_server(state: Arc<AppState>, addr: std::net::SocketAddr) -> Result<()> {
    info!("Starting metrics server on {}", addr);

    let app = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .route("/health", axum::routing::get(health_check))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind metrics listener on {addr}"))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .with_context(|| format!("Metrics server failed on {addr}"))
}

async fn health_check() -> &'static str {
    "OK"
}
