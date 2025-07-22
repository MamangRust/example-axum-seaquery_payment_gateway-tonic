use anyhow::{Context, Result};
use prometheus_client::registry::Registry;
use shared::{
    abstract_trait::DynJwtService,
    config::JwtConfig,
    utils::{Metrics, SystemMetrics, run_metrics_collector},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::{di::DependenciesInject, service::GrpcClients};

#[derive(Debug)]
pub struct AppState {
    pub registry: Arc<Mutex<Registry>>,
    pub jwt_config: DynJwtService,
    pub metrics: Arc<Mutex<Metrics>>,
    pub di_container: DependenciesInject,
    pub system_metrics: Arc<SystemMetrics>,
}

impl AppState {
    pub async fn new(jwt_secret: &str) -> Result<Self> {
        let jwt_config = Arc::new(JwtConfig::new(jwt_secret)) as DynJwtService;
        let registry = Arc::new(Mutex::new(Registry::default()));
        let metrics = Arc::new(Mutex::new(Metrics::new()));
        let system_metrics = Arc::new(SystemMetrics::new());

        registry.lock().await.register_metrics(&system_metrics);

        tokio::spawn(run_metrics_collector(system_metrics.clone()));

        let channel = Channel::from_static("http://payment-server:50051")
            .connect()
            .await
            .context("gRPC connection to payment-server:50051 failed")?;

        let clients = GrpcClients::init(channel).await;

        let di_container = {
            let mut registry = registry.lock().await;
            DependenciesInject::new(clients, metrics.clone(), &mut registry)
                .await
                .context("Failed to initialize dependency injection container")?
        };

        Ok(Self {
            registry,
            jwt_config,
            metrics,
            di_container,
            system_metrics,
        })
    }
}

trait MetricsRegister {
    fn register_metrics(&mut self, metrics: &SystemMetrics);
}

impl MetricsRegister for Registry {
    fn register_metrics(&mut self, metrics: &SystemMetrics) {
        metrics.register(self);
    }
}
