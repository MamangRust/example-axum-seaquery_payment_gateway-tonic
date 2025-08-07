use anyhow::{Context, Result};
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    abstract_trait::{
        DynAuthService, DynHashing, DynJwtService, DynSaldoRepository, DynSaldoService,
        DynTopupRepository, DynTopupService, DynTransferRepository, DynTransferService,
        DynUserRepository, DynUserService, DynWithdrawRepository, DynWithdrawService,
    },
    cache::CacheStore,
    config::{ConnectionPool, RedisClient, RedisConfig},
    repository::{
        saldo::SaldoRepository, topup::TopupRepository, transfer::TransferRepository,
        user::UserRepository, withdraw::WithdrawRepository,
    },
    service::{
        auth::AuthService, saldo::SaldoService, topup::TopupService, transfer::TransferService,
        user::UserService, withdraw::WithdrawService,
    },
    utils::Metrics,
};

#[derive(Clone)]
pub struct DependenciesInject {
    pub auth_service: DynAuthService,
    pub user_service: DynUserService,
    pub saldo_service: DynSaldoService,
    pub topup_service: DynTopupService,
    pub transfer_service: DynTransferService,
    pub withdraw_service: DynWithdrawService,
}

impl std::fmt::Debug for DependenciesInject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DependenciesInject")
            .field("auth_service", &"DynAuthService")
            .field("user_service", &"DynUserService")
            .field("saldo_service", &"DynSaldoService")
            .field("topup_service", &"DynTopupService")
            .field("transfer_service", &"DynTransferService")
            .field("withdraw_service", &"DynWithdrawService")
            .finish()
    }
}

impl DependenciesInject {
    pub async fn new(
        pool: ConnectionPool,
        hashing: DynHashing,
        jwt_config: DynJwtService,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Result<Self> {
        let config = RedisConfig {
            host: "redis".into(),
            port: 6379,
            db: 1,
            password: Some("dragon_knight".into()),
        };

        let redis = RedisClient::new(&config)
            .await
            .context("Failed to connect to Redis")
            .unwrap();

        redis.ping().context("Failed to ping Redis server").unwrap();

        let cache = Arc::new(CacheStore::new(redis.client.clone()));

        let user_repository = Arc::new(UserRepository::new(pool.clone())) as DynUserRepository;

        let user_service = Arc::new(
            UserService::new(
                user_repository.clone(),
                hashing.clone(),
                metrics.clone(),
                registry,
                cache.clone(),
            )
            .await,
        ) as DynUserService;

        let auth_service = Arc::new(
            AuthService::new(
                user_repository.clone(),
                hashing.clone(),
                jwt_config,
                metrics.clone(),
                registry,
                cache.clone(),
            )
            .await,
        ) as DynAuthService;

        let saldo_repository = Arc::new(SaldoRepository::new(pool.clone())) as DynSaldoRepository;

        let topup_repository = Arc::new(TopupRepository::new(pool.clone())) as DynTopupRepository;

        let transfer_repository =
            Arc::new(TransferRepository::new(pool.clone())) as DynTransferRepository;

        let withdraw_repository =
            Arc::new(WithdrawRepository::new(pool.clone())) as DynWithdrawRepository;

        let saldo_service = Arc::new(
            SaldoService::new(
                user_repository.clone(),
                saldo_repository.clone(),
                metrics.clone(),
                registry,
                cache.clone(),
            )
            .await,
        ) as DynSaldoService;

        let topup_service = Arc::new(
            TopupService::new(
                topup_repository.clone(),
                saldo_repository.clone(),
                user_repository.clone(),
                metrics.clone(),
                registry,
                cache.clone(),
            )
            .await,
        ) as DynTopupService;

        let transfer_service = Arc::new(
            TransferService::new(
                transfer_repository.clone(),
                saldo_repository.clone(),
                user_repository.clone(),
                metrics.clone(),
                registry,
                cache.clone(),
            )
            .await,
        ) as DynTransferService;

        let withdraw_service = Arc::new(
            WithdrawService::new(
                withdraw_repository.clone(),
                saldo_repository.clone(),
                user_repository.clone(),
                metrics.clone(),
                registry,
                cache.clone(),
            )
            .await,
        ) as DynWithdrawService;

        Ok(Self {
            auth_service,
            user_service,
            saldo_service,
            topup_service,
            transfer_service,
            withdraw_service,
        })
    }
}
