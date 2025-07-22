mod auth;
mod saldo;
mod topup;
mod transfer;
mod user;
mod withdraw;

pub use self::auth::AuthService;
pub use self::saldo::SaldoService;
pub use self::topup::TopupService;
pub use self::transfer::TransferService;
pub use self::user::UserService;
pub use self::withdraw::WithdrawService;

use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;

use genproto::{
    auth::auth_service_client::AuthServiceClient, saldo::saldo_service_client::SaldoServiceClient,
    topup::topup_service_client::TopupServiceClient,
    transfer::transfer_service_client::TransferServiceClient,
    user::user_service_client::UserServiceClient,
    withdraw::withdraw_service_client::WithdrawServiceClient,
};

#[derive(Clone)]
pub struct GrpcClients {
    pub auth: Arc<Mutex<AuthServiceClient<Channel>>>,
    pub saldo: Arc<Mutex<SaldoServiceClient<Channel>>>,
    pub topup: Arc<Mutex<TopupServiceClient<Channel>>>,
    pub transfer: Arc<Mutex<TransferServiceClient<Channel>>>,
    pub user: Arc<Mutex<UserServiceClient<Channel>>>,
    pub withdraw: Arc<Mutex<WithdrawServiceClient<Channel>>>,
}

impl GrpcClients {
    pub async fn init(channel: Channel) -> Self {
        Self {
            auth: Arc::new(Mutex::new(AuthServiceClient::new(channel.clone()))),
            user: Arc::new(Mutex::new(UserServiceClient::new(channel.clone()))),
            saldo: Arc::new(Mutex::new(SaldoServiceClient::new(channel.clone()))),
            topup: Arc::new(Mutex::new(TopupServiceClient::new(channel.clone()))),
            transfer: Arc::new(Mutex::new(TransferServiceClient::new(channel.clone()))),
            withdraw: Arc::new(Mutex::new(WithdrawServiceClient::new(channel))),
        }
    }
}
