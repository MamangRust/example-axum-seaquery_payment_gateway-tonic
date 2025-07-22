use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct Topup {
    pub topup_id: i32,
    pub user_id: i32,
    pub topup_no: String,
    pub topup_amount: i32,
    pub topup_method: String,
    pub topup_time: DateTime<Utc>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}
