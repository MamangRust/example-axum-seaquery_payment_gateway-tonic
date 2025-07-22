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
    utils::{Telemetry, init_logger},
};
use std::sync::Arc;
use tokio::net::TcpListener;

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

    let mytelemetry = Telemetry::new("myserver");
    let tracer_provider = mytelemetry.init_tracer();
    let meter_provider = mytelemetry.init_meter();
    let logger_provider = mytelemetry.init_logger();

    init_logger(logger_provider.clone(), "server");

    let config = Config::init().context("Failed to load configuration")?;

    let db_pool = ConnectionManager::new_pool(&config.database_url, config.run_migrations)
        .await
        .context("Failed to initialize database pool")?;

    let state = Arc::new(AppState::new(db_pool, &config.jwt_secret).await);

    let service_auth = service::auth::AuthServiceImpl::new(state.clone());
    let service_user = service::user::UserServiceImpl::new(state.clone());
    let service_topup = service::topup::TopupServiceImpl::new(state.clone());
    let service_saldo = service::saldo::SaldoServiceImpl::new(state.clone());
    let service_transfer = service::transfer::TransferServiceImpl::new(state.clone());
    let service_withdraw = service::withdraw::WithdrawServiceImpl::new(state.clone());

    let grpc_addr = "0.0.0.0:50051"
        .parse()
        .context("Failed to parse gRPC address")?;

    let grpc_server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(AuthServiceServer::new(service_auth))
            .add_service(UserServiceServer::new(service_user))
            .add_service(SaldoServiceServer::new(service_saldo))
            .add_service(TopupServiceServer::new(service_topup))
            .add_service(TransferServiceServer::new(service_transfer))
            .add_service(WithdrawServiceServer::new(service_withdraw))
            .serve(grpc_addr)
            .await
            .context("Failed to start gRPC server")
    });

    let app = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:8080")
        .await
        .context("Failed to bind Axum metrics listener")?;

    println!("gRPC Server running on 0.0.0.0:50051");
    println!("Metrics Server running on http://0.0.0.0:8080");

    let axum_server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .context("Axum server failed")
    });

    let grpc_result = grpc_server.await.context("gRPC task join error")?;
    let axum_result = axum_server.await.context("Axum task join error")?;

    grpc_result.context("gRPC server failed")?;
    axum_result.context("Axum server failed")?;

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
