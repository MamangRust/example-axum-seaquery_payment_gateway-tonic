use crate::service::{
    AuthService, GrpcClients, SaldoService, TopupService, TransferService, UserService,
    WithdrawService,
};
use shared::{
    abstract_trait::{
        DynAuthService, DynSaldoService, DynTopupService, DynTransferService, DynUserService,
        DynWithdrawService,
    },
    utils::Metrics,
};

use anyhow::Result;
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tokio::sync::Mutex;

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
        clients: GrpcClients,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Result<Self> {
        let auth_service: DynAuthService =
            Arc::new(AuthService::new(clients.auth, metrics.clone(), registry).await);
        let user_service: DynUserService =
            Arc::new(UserService::new(clients.user, metrics.clone(), registry).await);
        let saldo_service: DynSaldoService =
            Arc::new(SaldoService::new(clients.saldo, metrics.clone(), registry).await);
        let topup_service: DynTopupService =
            Arc::new(TopupService::new(clients.topup, metrics.clone(), registry).await);
        let transfer_service: DynTransferService =
            Arc::new(TransferService::new(clients.transfer, metrics.clone(), registry).await);
        let withdraw_service: DynWithdrawService =
            Arc::new(WithdrawService::new(clients.withdraw, metrics.clone(), registry).await);

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
