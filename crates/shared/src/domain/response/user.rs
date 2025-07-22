use crate::{model::user::User, utils::parse_datetime};
use chrono::{DateTime, Utc};
use genproto::user::UserResponse as UserResponseProto;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct UserResponse {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub noc_transfer: String,
    #[schema(format = "date-time")]
    pub created_at: Option<DateTime<Utc>>,

    #[schema(format = "date-time")]
    pub updated_at: Option<DateTime<Utc>>,
}

// dari database record ke response
impl From<User> for UserResponse {
    fn from(value: User) -> Self {
        UserResponse {
            id: value.user_id,
            firstname: value.firstname,
            lastname: value.lastname,
            email: value.email,
            noc_transfer: value.noc_transfer,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

// dari proto ke response
impl From<UserResponseProto> for UserResponse {
    fn from(value: UserResponseProto) -> Self {
        UserResponse {
            id: value.user_id,
            firstname: value.firstname,
            lastname: value.lastname,
            email: value.email,
            noc_transfer: value.noc_transfer,
            created_at: parse_datetime(&value.created_at),
            updated_at: parse_datetime(&value.updated_at),
        }
    }
}

// dari response ke proto
impl From<UserResponse> for UserResponseProto {
    fn from(value: UserResponse) -> Self {
        UserResponseProto {
            user_id: value.id,
            firstname: value.firstname,
            lastname: value.lastname,
            email: value.email,
            noc_transfer: value.noc_transfer,
            created_at: value.created_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            updated_at: value.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
        }
    }
}

// dari option proto ke response
impl From<Option<UserResponseProto>> for UserResponse {
    fn from(value: Option<UserResponseProto>) -> Self {
        match value {
            Some(proto) => proto.into(),
            None => UserResponse {
                id: 0,
                firstname: "".to_string(),
                lastname: "".to_string(),
                email: "".to_string(),
                noc_transfer: "".to_string(),
                created_at: None,
                updated_at: None,
            },
        }
    }
}
