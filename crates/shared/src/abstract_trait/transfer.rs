use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    domain::{
        request::{
            CreateTransferRequest, FindAllTransferRequest, UpdateTransferAmountRequest,
            UpdateTransferRequest,
        },
        response::{ApiResponse, ApiResponsePagination, ErrorResponse, transfer::TransferResponse},
    },
    model::transfer::Transfer,
    utils::AppError,
};

pub type DynTransferRepository = Arc<dyn TransferRepositoryTrait + Send + Sync>;
pub type DynTransferService = Arc<dyn TransferServiceTrait + Send + Sync>;

#[async_trait]
pub trait TransferRepositoryTrait {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Transfer>, i64), AppError>;
    async fn find_by_id(&self, id: i32) -> Result<Option<Transfer>, AppError>;
    async fn find_by_users(&self, id: i32) -> Result<Vec<Transfer>, AppError>;
    async fn find_by_user(&self, id: i32) -> Result<Option<Transfer>, AppError>;
    async fn create(&self, input: &CreateTransferRequest) -> Result<Transfer, AppError>;
    async fn update(&self, input: &UpdateTransferRequest) -> Result<Transfer, AppError>;
    async fn update_amount(
        &self,
        input: &UpdateTransferAmountRequest,
    ) -> Result<Transfer, AppError>;
    async fn delete(&self, id: i32) -> Result<(), AppError>;
}

#[async_trait]
pub trait TransferServiceTrait {
    async fn get_transfers(
        &self,
        req: &FindAllTransferRequest,
    ) -> Result<ApiResponsePagination<Vec<TransferResponse>>, ErrorResponse>;
    async fn get_transfer(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<TransferResponse>>, ErrorResponse>;
    async fn get_transfer_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<TransferResponse>>>, ErrorResponse>;
    async fn get_transfer_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<TransferResponse>>, ErrorResponse>;
    async fn create_transfer(
        &self,
        input: &CreateTransferRequest,
    ) -> Result<ApiResponse<TransferResponse>, ErrorResponse>;
    async fn update_transfer(
        &self,
        input: &UpdateTransferRequest,
    ) -> Result<ApiResponse<TransferResponse>, ErrorResponse>;
    async fn delete_transfer(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse>;
}
