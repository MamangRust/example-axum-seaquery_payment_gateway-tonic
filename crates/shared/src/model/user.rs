use chrono::{DateTime, Utc, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct User {
    pub user_id: i32,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub password: String,
    pub noc_transfer: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}
