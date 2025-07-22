use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    domain::{
        request::{CreateTopupRequest, FindAllTopupRequest, UpdateTopupAmount, UpdateTopupRequest},
        response::{ApiResponse, ApiResponsePagination, ErrorResponse, topup::TopupResponse},
    },
    model::topup::Topup,
    utils::AppError,
};

pub type DynTopupRepository = Arc<dyn TopupRepositoryTrait + Send + Sync>;
pub type DynTopupService = Arc<dyn TopupServiceTrait + Send + Sync>;

#[async_trait]
pub trait TopupRepositoryTrait {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Topup>, i64), AppError>;

    async fn find_by_id(&self, id: i32) -> Result<Option<Topup>, AppError>;
    async fn find_by_users(&self, id: i32) -> Result<Vec<Topup>, AppError>;
    async fn find_by_user(&self, id: i32) -> Result<Option<Topup>, AppError>;
    async fn create(&self, input: &CreateTopupRequest) -> Result<Topup, AppError>;
    async fn update(&self, input: &UpdateTopupRequest) -> Result<Topup, AppError>;
    async fn update_amount(&self, input: &UpdateTopupAmount) -> Result<Topup, AppError>;
    async fn delete(&self, id: i32) -> Result<(), AppError>;
}

#[async_trait]
pub trait TopupServiceTrait {
    async fn get_topups(
        &self,
        req: &FindAllTopupRequest,
    ) -> Result<ApiResponsePagination<Vec<TopupResponse>>, ErrorResponse>;
    async fn get_topup(&self, id: i32)
    -> Result<ApiResponse<Option<TopupResponse>>, ErrorResponse>;
    async fn get_topup_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<TopupResponse>>>, ErrorResponse>;
    async fn get_topup_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<TopupResponse>>, ErrorResponse>;
    async fn create_topup(
        &self,
        input: &CreateTopupRequest,
    ) -> Result<ApiResponse<TopupResponse>, ErrorResponse>;
    async fn update_topup(
        &self,
        input: &UpdateTopupRequest,
    ) -> Result<ApiResponse<TopupResponse>, ErrorResponse>;
    async fn delete_topup(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse>;
}
