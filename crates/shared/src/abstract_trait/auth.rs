use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::{
    request::auth::{LoginRequest, RegisterRequest},
    response::{ApiResponse, ErrorResponse, user::UserResponse},
};

pub type DynAuthService = Arc<dyn AuthServiceTrait + Send + Sync>;

#[async_trait]
pub trait AuthServiceTrait {
    async fn register_user(
        &self,
        input: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse>;
    async fn login_user(&self, input: &LoginRequest) -> Result<ApiResponse<String>, ErrorResponse>;
    async fn get_me(&self, id: i32) -> Result<ApiResponse<UserResponse>, ErrorResponse>;
}
