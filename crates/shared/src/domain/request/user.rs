use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Serialize, Deserialize, Clone, Debug, IntoParams)]
pub struct FindAllUserRequest {
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
pub struct CreateUserRequest {
    #[validate(length(min = 1, message = "First name is required"))]
    pub firstname: String,

    #[validate(length(min = 1, message = "Last name is required"))]
    pub lastname: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub password: String,

    #[validate(length(min = 6, message = "Confirm password must be at least 6 characters"))]
    #[validate(must_match(other = "password"))]
    pub confirm_password: String,

    pub noc_transfer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateUserRequest {
    #[validate(range(min = 1))]
    pub id: i32,

    #[validate(length(min = 1, message = "First name is required"))]
    pub firstname: Option<String>,

    #[validate(length(min = 1, message = "Last name is required"))]
    pub lastname: Option<String>,

    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,

    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub password: String,

    #[validate(length(min = 6, message = "Confirm password must be at least 6 characters"))]
    #[validate(must_match(other = "password"))]
    pub confirm_password: String,
}
