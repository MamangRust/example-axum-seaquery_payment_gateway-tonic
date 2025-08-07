use anyhow::{Context, Result};
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    abstract_trait::{DynHashing, DynJwtService},
    config::{ConnectionPool, Hashing, JwtConfig},
    utils::{DependenciesInject, Metrics, SystemMetrics, run_metrics_collector},
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub di_container: DependenciesInject,
    pub jwt_config: DynJwtService,
    pub registry: Arc<Mutex<Registry>>,
    pub metrics: Arc<Mutex<Metrics>>,
    pub system_metrics: Arc<SystemMetrics>,
}

impl AppState {
    pub async fn new(pool: ConnectionPool, jwt_secret: &str) -> Result<Self> {
        let jwt_config = Arc::new(JwtConfig::new(jwt_secret)) as DynJwtService;
        let hashing = Arc::new(Hashing::new()) as DynHashing;
        let registry = Arc::new(Mutex::new(Registry::default()));
        let metrics = Arc::new(Mutex::new(Metrics::new()));
        let system_metrics = Arc::new(SystemMetrics::new());

        registry.lock().await.register_metrics(&system_metrics);

        tokio::spawn(run_metrics_collector(system_metrics.clone()));

        let di_container = {
            let mut registry_guard = registry.lock().await;
            DependenciesInject::new(
                pool,
                hashing,
                jwt_config.clone(),
                metrics.clone(),
                &mut registry_guard,
            )
            .await
            .context("Failed to initialize dependency injection container")?
        };

        Ok(Self {
            registry,
            di_container,
            jwt_config,
            metrics,
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
