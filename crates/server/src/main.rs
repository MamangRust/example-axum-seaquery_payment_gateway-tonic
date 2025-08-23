use anyhow::{Context, Result};
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderValue, StatusCode, header::CONTENT_TYPE},
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
    utils::Telemetry,
    utils::init_logger,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

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
            HeaderValue::from_static("application/openmetrics-text; version=1.0.0; charset=utf-8"),
        )
        .body(Body::from(buffer))
        .unwrap()
}

async fn health_check() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let config = Config::init().context("Failed to load configuration")?;
    let server_config = ServerConfig::from_config(&config)?;

    let telemetry = Telemetry::new("payment-service", "http://otel-collector:4317".to_string());

    let logger_provider = telemetry.init_logger();
    let _meter_provider = telemetry.init_meter();
    let _tracer_provider = telemetry.init_tracer();

    init_logger(logger_provider.clone(), "payment-service");

    info!("🚀 Starting Payment Service initialization...");

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

    let (shutdown_tx, _) = broadcast::channel(1);

    // 🛰️ gRPC server
    let grpc_addr = server_config.grpc_addr;
    let grpc_shutdown_rx = shutdown_tx.subscribe();
    let grpc_handle = tokio::spawn(async move {
        loop {
            match start_grpc_server(services.clone(), grpc_addr, grpc_shutdown_rx.resubscribe())
                .await
            {
                Ok(()) => {
                    info!("gRPC server stopped gracefully");
                    break;
                }
                Err(e) => {
                    error!("❌ gRPC server failed: {e}. Restarting in 5s...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    let metrics_addr = server_config.metrics_addr;
    let state_clone = state.clone();
    let metrics_shutdown_rx = shutdown_tx.subscribe();
    let metrics_handle = tokio::spawn(async move {
        loop {
            info!("🔧 Starting metrics server on {metrics_addr}");
            match start_metrics_server(
                state_clone.clone(),
                metrics_addr,
                metrics_shutdown_rx.resubscribe(),
            )
            .await
            {
                Ok(()) => {
                    info!("Metrics server stopped gracefully");
                    break;
                }
                Err(e) => {
                    error!("❌ Metrics server failed: {e}. Retrying in 3s...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                }
            }
        }
    });

    let signal_shutdown_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("🛑 Shutdown signal received.");
                let _ = signal_shutdown_tx.send(());
            }
            Err(e) => {
                error!("Failed to listen for shutdown signal: {}", e);
            }
        }
    });

    let mut shutdown_rx = shutdown_tx.subscribe();
    let _ = shutdown_rx.recv().await;

    info!("🛑 Shutting down all servers...");

    let shutdown_timeout = tokio::time::Duration::from_secs(30);
    let shutdown_result = tokio::time::timeout(shutdown_timeout, async {
        let _ = tokio::join!(grpc_handle, metrics_handle);
    })
    .await;

    match shutdown_result {
        Ok(()) => info!("✅ All servers shutdown gracefully"),
        Err(_) => {
            warn!("⚠️  Shutdown timeout reached, forcing exit");
        }
    }

    if let Err(e) = telemetry.shutdown().await {
        error!("Failed to shutdown telemetry: {}", e);
    }

    info!("✅ Payment Service shutdown complete.");

    Ok(())
}

async fn start_grpc_server(
    services: ServiceContainer,
    addr: std::net::SocketAddr,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("📡 Starting gRPC server on {addr}");

    let shutdown_future = async move {
        let _ = shutdown_rx.recv().await;
        info!("gRPC server received shutdown signal");
    };

    tonic::transport::Server::builder()
        .add_service(AuthServiceServer::new(services.auth))
        .add_service(UserServiceServer::new(services.user))
        .add_service(SaldoServiceServer::new(services.saldo))
        .add_service(TopupServiceServer::new(services.topup))
        .add_service(TransferServiceServer::new(services.transfer))
        .add_service(WithdrawServiceServer::new(services.withdraw))
        .serve_with_shutdown(addr, shutdown_future)
        .await
        .with_context(|| format!("gRPC server failed to start on {addr}"))
}

async fn start_metrics_server(
    state: Arc<AppState>,
    addr: std::net::SocketAddr,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    info!("Starting metrics server on {}", addr);

    let app = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .route("/health", axum::routing::get(health_check))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind metrics listener on {addr}"))?;

    let shutdown_future = async move {
        let _ = shutdown_rx.recv().await;
        info!("Metrics server received shutdown signal");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_future)
        .await
        .with_context(|| format!("Metrics server failed on {addr}"))
}
