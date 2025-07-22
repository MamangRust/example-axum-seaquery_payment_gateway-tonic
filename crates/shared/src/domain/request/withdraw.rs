use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::{Validate, ValidationError};

#[derive(Serialize, Deserialize, Clone, Debug, IntoParams)]
pub struct FindAllWithdrawRequest {
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
#[validate(schema(function = "validate_create_not_future"))]
pub struct CreateWithdrawRequest {
    #[validate(range(min = 1, message = "User ID must be positive"))]
    pub user_id: i32,

    #[validate(range(min = 50001, message = "Withdraw amount must be at least 50,001"))]
    pub withdraw_amount: i32,

    pub withdraw_time: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
#[validate(schema(function = "validate_update_not_future"))]
pub struct UpdateWithdrawRequest {
    #[validate(range(min = 1, message = "User ID must be positive"))]
    pub user_id: i32,

    #[validate(range(min = 1, message = "Withdraw ID must be positive"))]
    pub withdraw_id: i32,

    #[validate(range(min = 50001, message = "Withdraw amount must be at least 50,001"))]
    pub withdraw_amount: i32,

    pub withdraw_time: DateTime<Utc>,
}

fn validate_create_not_future(data: &CreateWithdrawRequest) -> Result<(), ValidationError> {
    if data.withdraw_time > Utc::now() {
        let mut error = ValidationError::new("withdraw_time_in_future");
        error.add_param("value".into(), &data.withdraw_time.to_string());
        return Err(error);
    }
    Ok(())
}

fn validate_update_not_future(data: &UpdateWithdrawRequest) -> Result<(), ValidationError> {
    if data.withdraw_time > Utc::now() {
        let mut error = ValidationError::new("withdraw_time_in_future");
        error.add_param("value".into(), &data.withdraw_time.to_string());
        return Err(error);
    }
    Ok(())
}
