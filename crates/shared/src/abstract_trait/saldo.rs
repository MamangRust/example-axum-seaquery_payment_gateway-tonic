use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    domain::{
        request::{
            CreateSaldoRequest, FindAllSaldoRequest, UpdateSaldoBalance, UpdateSaldoRequest,
            UpdateSaldoWithdraw,
        },
        response::{ApiResponse, ApiResponsePagination, ErrorResponse, saldo::SaldoResponse},
    },
    model::saldo::Saldo,
    utils::AppError,
};

pub type DynSaldoRepository = Arc<dyn SaldoRepositoryTrait + Send + Sync>;
pub type DynSaldoService = Arc<dyn SaldoServiceTrait + Send + Sync>;

#[async_trait]
pub trait SaldoRepositoryTrait {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Saldo>, i64), AppError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<Saldo>, AppError>;

    async fn find_by_users_id(&self, id: i32) -> Result<Vec<Saldo>, AppError>;
    async fn find_by_user_id(&self, id: i32) -> Result<Option<Saldo>, AppError>;
    async fn create(&self, input: &CreateSaldoRequest) -> Result<Saldo, AppError>;
    async fn update(&self, input: &UpdateSaldoRequest) -> Result<Saldo, AppError>;
    async fn update_balance(&self, input: &UpdateSaldoBalance) -> Result<Saldo, AppError>;
    async fn update_saldo_withdraw(&self, input: &UpdateSaldoWithdraw) -> Result<Saldo, AppError>;
    async fn delete(&self, id: i32) -> Result<(), AppError>;
}

#[async_trait]
pub trait SaldoServiceTrait {
    async fn get_saldos(
        &self,
        req: &FindAllSaldoRequest,
    ) -> Result<ApiResponsePagination<Vec<SaldoResponse>>, ErrorResponse>;
    async fn get_saldo(&self, id: i32)
    -> Result<ApiResponse<Option<SaldoResponse>>, ErrorResponse>;
    async fn get_saldo_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<SaldoResponse>>>, ErrorResponse>;
    async fn get_saldo_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<SaldoResponse>>, ErrorResponse>;
    async fn create_saldo(
        &self,
        input: &CreateSaldoRequest,
    ) -> Result<ApiResponse<SaldoResponse>, ErrorResponse>;
    async fn update_saldo(
        &self,
        input: &UpdateSaldoRequest,
    ) -> Result<ApiResponse<SaldoResponse>, ErrorResponse>;
    async fn delete_saldo(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse>;
}
