use crate::{model::saldo::Saldo, utils::parse_datetime};
use chrono::{DateTime, Utc};
use genproto::saldo::SaldoResponse as SaldoResponseProto;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct SaldoResponse {
    pub id: i32,
    pub user_id: i32,
    pub total_balance: i32,
    pub withdraw_amount: Option<i32>,
    #[schema(format = "date-time")]
    pub withdraw_time: Option<DateTime<Utc>>,
    #[schema(format = "date-time")]
    pub created_at: Option<DateTime<Utc>>,
    #[schema(format = "date-time")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Saldo> for SaldoResponse {
    fn from(value: Saldo) -> Self {
        SaldoResponse {
            id: value.saldo_id,
            user_id: value.user_id,
            total_balance: value.total_balance,
            withdraw_amount: value.withdraw_amount,
            withdraw_time: value.withdraw_time,
            created_at: value
                .created_at
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            updated_at: value
                .updated_at
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        }
    }
}

impl From<SaldoResponseProto> for SaldoResponse {
    fn from(value: SaldoResponseProto) -> Self {
        SaldoResponse {
            id: value.saldo_id,
            user_id: value.user_id,
            total_balance: value.total_balance,
            withdraw_amount: Some(value.withdraw_amount),
            withdraw_time: parse_datetime(&value.withdraw_time),
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

impl From<SaldoResponse> for SaldoResponseProto {
    fn from(value: SaldoResponse) -> Self {
        SaldoResponseProto {
            saldo_id: value.id,
            user_id: value.user_id,
            total_balance: value.total_balance,
            withdraw_amount: value.withdraw_amount.unwrap_or_default(),
            withdraw_time: value
                .withdraw_time
                .map(|d| d.to_rfc3339())
                .unwrap_or_default(),
            created_at: value.created_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            updated_at: value.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
        }
    }
}

impl From<Option<SaldoResponseProto>> for SaldoResponse {
    fn from(value: Option<SaldoResponseProto>) -> Self {
        match value {
            Some(proto) => proto.into(),
            None => SaldoResponse {
                id: 0,
                user_id: 0,
                total_balance: 0,
                withdraw_amount: None,
                withdraw_time: None,
                created_at: None,
                updated_at: None,
            },
        }
    }
}
