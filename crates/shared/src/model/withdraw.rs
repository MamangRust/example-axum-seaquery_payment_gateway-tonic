use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct Withdraw {
    pub withdraw_id: i32,
    pub user_id: i32,
    pub withdraw_amount: i32,
    pub withdraw_time: DateTime<Utc>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}
