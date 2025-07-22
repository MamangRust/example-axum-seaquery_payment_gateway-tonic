use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    domain::{
        request::{CreateWithdrawRequest, FindAllWithdrawRequest, UpdateWithdrawRequest},
        response::{ApiResponse, ApiResponsePagination, ErrorResponse, withdraw::WithdrawResponse},
    },
    model::withdraw::Withdraw,
    utils::AppError,
};

pub type DynWithdrawRepository = Arc<dyn WithdrawRepositoryTrait + Send + Sync>;
pub type DynWithdrawService = Arc<dyn WithdrawServiceTrait + Send + Sync>;

#[async_trait]
pub trait WithdrawRepositoryTrait {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Withdraw>, i64), AppError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<Withdraw>, AppError>;
    async fn find_by_users(&self, id: i32) -> Result<Vec<Withdraw>, AppError>;
    async fn find_by_user(&self, id: i32) -> Result<Option<Withdraw>, AppError>;
    async fn create(&self, input: &CreateWithdrawRequest) -> Result<Withdraw, AppError>;
    async fn update(&self, input: &UpdateWithdrawRequest) -> Result<Withdraw, AppError>;
    async fn delete(&self, id: i32) -> Result<(), AppError>;
}

#[async_trait]
pub trait WithdrawServiceTrait {
    async fn get_withdraws(
        &self,
        req: &FindAllWithdrawRequest,
    ) -> Result<ApiResponsePagination<Vec<WithdrawResponse>>, ErrorResponse>;
    async fn get_withdraw(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<WithdrawResponse>>, ErrorResponse>;
    async fn get_withdraw_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<WithdrawResponse>>>, ErrorResponse>;
    async fn get_withdraw_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<WithdrawResponse>>, ErrorResponse>;
    async fn create_withdraw(
        &self,
        input: &CreateWithdrawRequest,
    ) -> Result<ApiResponse<WithdrawResponse>, ErrorResponse>;
    async fn update_withdraw(
        &self,
        input: &UpdateWithdrawRequest,
    ) -> Result<ApiResponse<WithdrawResponse>, ErrorResponse>;
    async fn delete_withdraw(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse>;
}
