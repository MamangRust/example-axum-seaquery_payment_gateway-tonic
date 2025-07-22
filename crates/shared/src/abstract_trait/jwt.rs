use crate::utils::AppError;
use async_trait::async_trait;
use std::sync::Arc;

use anyhow::Result;

#[async_trait]
pub trait JwtServiceTrait: Send + Sync + std::fmt::Debug {
    fn generate_token(&self, user_id: i64) -> Result<String, AppError>;
    fn verify_token(&self, token: &str) -> Result<i64, AppError>;
}

pub type DynJwtService = Arc<dyn JwtServiceTrait + Send + Sync>;
