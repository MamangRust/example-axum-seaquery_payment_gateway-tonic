use anyhow::{Context, Result};
use shared::config::Config;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub grpc_addr: std::net::SocketAddr,
    pub metrics_addr: std::net::SocketAddr,
    pub database_url: String,
    pub jwt_secret: String,
    pub run_migrations: bool,
}

impl ServerConfig {
    pub fn from_config(config: &Config) -> Result<Self> {
        Ok(Self {
            grpc_addr: format!("0.0.0.0:{}", config.grpc_port)
                .parse()
                .context("Invalid gRPC address")?,
            metrics_addr: format!("0.0.0.0:{}", config.metric_port)
                .parse()
                .context("Invalid metrics address")?,
            database_url: config.database_url.clone(),
            jwt_secret: config.jwt_secret.clone(),
            run_migrations: config.run_migrations,
        })
    }
}
