use crate::{model::withdraw::Withdraw, utils::parse_datetime};
use chrono::{DateTime, Utc};
use genproto::withdraw::WithdrawResponse as WithdrawResponseProto;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct WithdrawResponse {
    pub withdraw_id: i32,
    pub user_id: i32,
    pub withdraw_amount: i32,
    #[schema(format = "date-time")]
    pub withdraw_time: DateTime<Utc>,
    #[schema(format = "date-time")]
    pub created_at: Option<DateTime<Utc>>,
    #[schema(format = "date-time")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Withdraw> for WithdrawResponse {
    fn from(value: Withdraw) -> Self {
        WithdrawResponse {
            withdraw_id: value.withdraw_id,
            user_id: value.user_id,
            withdraw_amount: value.withdraw_amount,
            withdraw_time: DateTime::from_naive_utc_and_offset(value.withdraw_time, Utc),
            created_at: value
                .created_at
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            updated_at: value
                .updated_at
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        }
    }
}

impl From<WithdrawResponse> for WithdrawResponseProto {
    fn from(value: WithdrawResponse) -> Self {
        WithdrawResponseProto {
            withdraw_id: value.withdraw_id,
            user_id: value.user_id,
            withdraw_amount: value.withdraw_amount,
            withdraw_time: value.withdraw_time.to_rfc3339(),
            created_at: value
                .created_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
            updated_at: value
                .updated_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        }
    }
}

impl From<WithdrawResponseProto> for WithdrawResponse {
    fn from(value: WithdrawResponseProto) -> Self {
        let now = Utc::now();

        WithdrawResponse {
            withdraw_id: value.withdraw_id,
            user_id: value.user_id,
            withdraw_amount: value.withdraw_amount,
            withdraw_time: parse_datetime(&value.withdraw_time).unwrap_or(now),
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

impl From<Option<WithdrawResponseProto>> for WithdrawResponse {
    fn from(value: Option<WithdrawResponseProto>) -> Self {
        match value {
            Some(value) => value.into(),
            None => WithdrawResponse {
                withdraw_id: 0,
                user_id: 0,
                withdraw_amount: 0,
                withdraw_time: Utc::now(),
                created_at: None,
                updated_at: None,
            },
        }
    }
}
