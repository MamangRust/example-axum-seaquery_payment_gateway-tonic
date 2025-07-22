use crate::{
    middleware::{jwt, validate::SimpleValidatedJson},
    state::AppState,
};
use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde_json::json;
use shared::domain::{
    request::{FindAllUserRequest, RegisterRequest, UpdateUserRequest},
    response::{ApiResponse, ApiResponsePagination, user::UserResponse},
};
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/users",
    tag = "User",
    security(
        ("bearer_auth" = [])
    ),
    params(FindAllUserRequest),
    responses(
        (status = 200, description = "List of user records", body = ApiResponsePagination<Vec<UserResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn get_users(
    State(data): State<Arc<AppState>>,
    Query(params): Query<FindAllUserRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.user_service.get_users(&params).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/users/{id}",
    tag = "User",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User details retrieved successfully", body = ApiResponse<Option<UserResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 404, description = "User not found", body = String),
    )
)]
pub async fn get_user(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.user_service.get_user(id).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    post,
    path = "/api/users",
    tag = "User",
    security(
        ("bearer_auth" = [])
    ),
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User account created successfully", body = ApiResponse<UserResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn create_user(
    State(data): State<Arc<AppState>>,
    SimpleValidatedJson(body): SimpleValidatedJson<RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.user_service.create_user(&body).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(json!(response)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    put,
    path = "/api/users/{id}",
    tag = "User",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User record updated successfully", body = ApiResponse<UserResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn update_user(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateUserRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    body.id = id;

    match data.di_container.user_service.update_user(&body).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    tag = "User",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User record deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn delete_user(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.user_service.delete_user(id).await {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "User deleted successfully"
            })),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

pub fn users_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/users", get(get_users))
        .route("/api/users/{id}", get(get_user))
        .route("/api/users", post(create_user))
        .route("/api/users/{id}", put(update_user))
        .route("/api/users/{id}", delete(delete_user))
        .route_layer(middleware::from_fn_with_state(app_state.clone(), jwt::auth))
        .with_state(app_state.clone())
}
