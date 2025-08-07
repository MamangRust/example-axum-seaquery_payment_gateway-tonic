use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct Transfer {
    pub transfer_id: i32,
    pub transfer_from: i32,
    pub transfer_to: i32,
    pub transfer_amount: i32,
    pub transfer_time: NaiveDateTime,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}
