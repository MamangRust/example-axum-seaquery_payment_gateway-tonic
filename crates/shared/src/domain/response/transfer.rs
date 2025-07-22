use crate::{model::transfer::Transfer, utils::parse_datetime};
use chrono::{DateTime, Utc};
use genproto::transfer::TransferResponse as TransferResponseProto;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct TransferResponse {
    pub transfer_id: i32,
    pub transfer_from: i32,
    pub transfer_to: i32,
    pub transfer_amount: i32,
    pub transfer_time: DateTime<Utc>,

    #[schema(format = "date-time")]
    pub created_at: Option<DateTime<Utc>>,

    #[schema(format = "date-time")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Transfer> for TransferResponse {
    fn from(value: Transfer) -> Self {
        TransferResponse {
            transfer_id: value.transfer_id,
            transfer_from: value.transfer_from,
            transfer_to: value.transfer_to,
            transfer_amount: value.transfer_amount,
            transfer_time: value.transfer_time,
            created_at: value
                .created_at
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
            updated_at: value
                .updated_at
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc)),
        }
    }
}

impl From<TransferResponseProto> for TransferResponse {
    fn from(value: TransferResponseProto) -> Self {
        TransferResponse {
            transfer_id: value.transfer_id,
            transfer_from: value.transfer_from,
            transfer_to: value.transfer_to,
            transfer_amount: value.transfer_amount,
            transfer_time: DateTime::parse_from_rfc3339(&value.transfer_time)
                .unwrap()
                .with_timezone(&Utc),
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

impl From<TransferResponse> for TransferResponseProto {
    fn from(value: TransferResponse) -> Self {
        TransferResponseProto {
            transfer_id: value.transfer_id,
            transfer_from: value.transfer_from,
            transfer_to: value.transfer_to,
            transfer_amount: value.transfer_amount,
            transfer_time: value.transfer_time.to_rfc3339(),
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

impl From<Option<TransferResponseProto>> for TransferResponse {
    fn from(value: Option<TransferResponseProto>) -> Self {
        match value {
            Some(proto) => proto.into(),
            None => TransferResponse {
                transfer_id: 0,
                transfer_from: 0,
                transfer_to: 0,
                transfer_amount: 0,
                transfer_time: Utc::now(),
                created_at: None,
                updated_at: None,
            },
        }
    }
}
