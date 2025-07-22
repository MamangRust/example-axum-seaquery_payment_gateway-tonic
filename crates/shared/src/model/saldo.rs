use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct Saldo {
    pub saldo_id: i32,
    pub user_id: i32,
    pub total_balance: i32,
    pub withdraw_amount: Option<i32>,
    pub withdraw_time: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
