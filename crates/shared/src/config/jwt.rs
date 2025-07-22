use crate::{abstract_trait::JwtServiceTrait, utils::AppError};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::ErrorKind as JwtError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub exp: usize,
    pub iat: usize,
}

impl Claims {
    pub fn new(user_id: i64, exp: usize, iat: usize) -> Self {
        Claims { user_id, exp, iat }
    }
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub jwt_secret: String,
}

impl JwtConfig {
    pub fn new(jwt_secret: &str) -> Self {
        JwtConfig {
            jwt_secret: jwt_secret.to_string(),
        }
    }
}

#[async_trait]
impl JwtServiceTrait for JwtConfig {
    fn generate_token(&self, user_id: i64) -> Result<String, AppError> {
        let now = Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + Duration::minutes(60)).timestamp() as usize;

        let claims = Claims::new(user_id, exp, iat);

        match encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        ) {
            Ok(token) => Ok(token),
            Err(err) => Err(AppError::TokenGenerationError(err)),
        }
    }

    fn verify_token(&self, token: &str) -> Result<i64, AppError> {
        let decoding_key = DecodingKey::from_secret(self.jwt_secret.as_ref());

        match decode::<Claims>(token, &decoding_key, &Validation::default()) {
            Ok(token_data) => {
                let current_time = Utc::now().timestamp() as usize;

                if token_data.claims.exp >= current_time {
                    Ok(token_data.claims.user_id)
                } else {
                    Err(AppError::TokenExpiredError)
                }
            }
            Err(err) => {
                if let JwtError::ExpiredSignature = err.kind() {
                    Err(AppError::TokenExpiredError)
                } else {
                    eprintln!("Error decoding token: {err:?}");
                    Err(AppError::TokenValidationError)
                }
            }
        }
    }
}
