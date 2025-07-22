use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Serialize, Deserialize, Clone, Debug, IntoParams)]
pub struct FindAllTransferRequest {
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
pub struct CreateTransferRequest {
    #[validate(range(min = 1, message = "Transfer from must be a positive integer"))]
    pub transfer_from: i32,

    #[validate(range(min = 1, message = "Transfer to must be a positive integer"))]
    pub transfer_to: i32,

    #[validate(range(min = 50000, message = "Transfer amount must be at least 50,000"))]
    pub transfer_amount: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateTransferRequest {
    #[validate(range(min = 1, message = "Transfer ID must be a positive integer"))]
    pub transfer_id: i32,

    #[validate(range(min = 1, message = "Transfer from must be a positive integer"))]
    pub transfer_from: i32,

    #[validate(range(min = 1, message = "Transfer to must be a positive integer"))]
    pub transfer_to: i32,

    #[validate(range(min = 50000, message = "Transfer amount must be at least 50,000"))]
    pub transfer_amount: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateTransferAmountRequest {
    #[validate(range(min = 1, message = "Transfer ID must be a positive integer"))]
    pub transfer_id: i32,

    #[validate(range(min = 50000, message = "Transfer amount must be at least 50,000"))]
    pub transfer_amount: i32,
}
