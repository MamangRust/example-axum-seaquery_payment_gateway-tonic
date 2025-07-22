use crate::{model::topup::Topup, utils::parse_datetime};
use chrono::{DateTime, Utc};
use genproto::topup::TopupResponse as TopupResponseProto;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct TopupResponse {
    pub topup_id: i32,
    pub user_id: i32,
    pub topup_no: String,
    pub topup_amount: i32,
    pub topup_method: String,
    pub topup_time: DateTime<Utc>,
    #[schema(format = "date-time")]
    pub created_at: Option<DateTime<Utc>>,
    #[schema(format = "date-time")]
    pub updated_at: Option<DateTime<Utc>>,
}

// dari model ke response
impl From<Topup> for TopupResponse {
    fn from(value: Topup) -> Self {
        TopupResponse {
            topup_id: value.topup_id,
            user_id: value.user_id,
            topup_no: value.topup_no,
            topup_amount: value.topup_amount,
            topup_method: value.topup_method,
            topup_time: value.topup_time,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

// dari proto ke response
impl From<TopupResponseProto> for TopupResponse {
    fn from(value: TopupResponseProto) -> Self {
        TopupResponse {
            topup_id: value.topup_id,
            user_id: value.user_id,
            topup_no: value.topup_no,
            topup_amount: value.topup_amount,
            topup_method: value.topup_method,
            topup_time: DateTime::parse_from_rfc3339(&value.topup_time)
                .unwrap()
                .with_timezone(&Utc),
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

// dari response ke proto
impl From<TopupResponse> for TopupResponseProto {
    fn from(value: TopupResponse) -> Self {
        TopupResponseProto {
            topup_id: value.topup_id,
            user_id: value.user_id,
            topup_no: value.topup_no,
            topup_amount: value.topup_amount,
            topup_method: value.topup_method,
            topup_time: value.topup_time.to_rfc3339(),
            created_at: value.created_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            updated_at: value.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
        }
    }
}

// dari option proto ke response
impl From<Option<TopupResponseProto>> for TopupResponse {
    fn from(value: Option<TopupResponseProto>) -> Self {
        match value {
            Some(proto) => proto.into(),
            None => TopupResponse {
                topup_id: 0,
                user_id: 0,
                topup_no: String::new(),
                topup_amount: 0,
                topup_method: String::new(),
                topup_time: Utc::now(),
                created_at: None,
                updated_at: None,
            },
        }
    }
}
