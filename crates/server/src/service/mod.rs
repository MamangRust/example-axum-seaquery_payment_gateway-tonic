mod auth;
mod saldo;
mod topup;
mod transfer;
mod user;
mod withdraw;

use std::sync::Arc;

use shared::state::AppState;

use self::auth::AuthServiceImpl;
use self::saldo::SaldoServiceImpl;
use self::topup::TopupServiceImpl;
use self::transfer::TransferServiceImpl;
use self::user::UserServiceImpl;
use self::withdraw::WithdrawServiceImpl;

#[derive(Clone)]
pub struct ServiceContainer {
    pub auth: AuthServiceImpl,
    pub user: UserServiceImpl,
    pub topup: TopupServiceImpl,
    pub saldo: SaldoServiceImpl,
    pub transfer: TransferServiceImpl,
    pub withdraw: WithdrawServiceImpl,
}

impl ServiceContainer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            auth: AuthServiceImpl::new(state.clone()),
            user: UserServiceImpl::new(state.clone()),
            topup: TopupServiceImpl::new(state.clone()),
            saldo: SaldoServiceImpl::new(state.clone()),
            transfer: TransferServiceImpl::new(state.clone()),
            withdraw: WithdrawServiceImpl::new(state.clone()),
        }
    }
}
