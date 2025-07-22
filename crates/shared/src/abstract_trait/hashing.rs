use crate::utils::AppError;
use async_trait::async_trait;
use bcrypt::BcryptError;

use anyhow::Result;

use std::sync::Arc;

#[async_trait]
pub trait HashingTrait: Send + Sync {
    async fn hash_password(&self, password: &str) -> Result<String, BcryptError>;
    async fn compare_password(&self, hashed_password: &str, password: &str)
    -> Result<(), AppError>;
}

pub type DynHashing = Arc<dyn HashingTrait + Send + Sync>;
