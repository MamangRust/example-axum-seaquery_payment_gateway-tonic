use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 2, message = "First name must be at least 2 characters"))]
    pub firstname: String,

    #[validate(length(min = 2, message = "Last name must be at least 2 characters"))]
    pub lastname: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    #[validate(length(min = 6, message = "Confirm password must be at least 6 characters"))]
    #[validate(must_match(other = "password"))]
    pub confirm_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}
