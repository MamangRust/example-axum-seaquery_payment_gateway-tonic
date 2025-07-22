use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Serialize, Deserialize, Clone, Debug, IntoParams)]
pub struct FindAllTopupRequest {
    #[serde(default = "default_page")]
    pub page: i32,

    #[serde(default = "default_page_size")]
    pub page_size: i32,

    #[serde(default)]
    pub search: String,
}

fn default_page() -> i32 {
    1
}

fn default_page_size() -> i32 {
    10
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateTopupRequest {
    #[validate(range(min = 1))]
    pub user_id: i32,

    #[validate(length(min = 1, message = "Top-up number is required"))]
    pub topup_no: String,

    #[validate(range(min = 1, message = "Top-up amount must be at least 1"))]
    pub topup_amount: i32,

    #[validate(length(min = 1, message = "Top-up method is required"))]
    pub topup_method: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateTopupRequest {
    #[validate(range(min = 1))]
    pub user_id: i32,

    #[validate(range(min = 1))]
    pub topup_id: i32,

    #[validate(range(min = 1, message = "Top-up amount must be at least 1"))]
    pub topup_amount: i32,

    #[validate(length(min = 1, message = "Top-up method is required"))]
    pub topup_method: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateTopupAmount {
    #[validate(range(min = 1, message = "Top-up ID must be a positive integer"))]
    pub topup_id: i32,

    #[validate(range(min = 1, message = "Top-up amount must be at least 1"))]
    pub topup_amount: i32,
}
