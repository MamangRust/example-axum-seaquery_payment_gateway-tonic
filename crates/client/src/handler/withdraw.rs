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
    request::{CreateWithdrawRequest, FindAllWithdrawRequest, UpdateWithdrawRequest},
    response::{ApiResponse, ApiResponsePagination, withdraw::WithdrawResponse},
};
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/withdraws",
    tag = "Withdraw",
    security(
        ("bearer_auth" = [])
    ),
    params(FindAllWithdrawRequest),
    responses(
        (status = 200, description = "List of withdrawals", body = ApiResponsePagination<Vec<WithdrawResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn get_withdraws(
    State(data): State<Arc<AppState>>,
    Query(params): Query<FindAllWithdrawRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data
        .di_container
        .withdraw_service
        .get_withdraws(&params)
        .await
    {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/withdraws/{id}",
    tag = "Withdraw",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Withdrawal ID")
    ),
    responses(
        (status = 200, description = "Withdrawal details retrieved successfully", body = ApiResponse<Option<WithdrawResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 404, description = "Withdrawal not found", body = String),
    )
)]
pub async fn get_withdraw(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.withdraw_service.get_withdraw(id).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/withdraws/users/{id}",
    tag = "Withdraw",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "List of user withdrawals", body = ApiResponse<Option<Vec<WithdrawResponse>>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 404, description = "Withdrawals not found", body = String),
    )
)]
pub async fn get_withdraw_users(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data
        .di_container
        .withdraw_service
        .get_withdraw_users(id)
        .await
    {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/withdraws/user/{id}",
    tag = "Withdraw",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User withdrawal details", body = ApiResponse<Option<WithdrawResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn get_withdraw_user(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data
        .di_container
        .withdraw_service
        .get_withdraw_user(id)
        .await
    {
        Ok(saldo) => Ok((StatusCode::OK, Json(json!(saldo)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    post,
    path = "/api/withdraws",
    tag = "Withdraw",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateWithdrawRequest,
    responses(
        (status = 201, description = "Withdrawal request created successfully", body = ApiResponse<WithdrawResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn create_withdraw(
    State(data): State<Arc<AppState>>,
    SimpleValidatedJson(body): SimpleValidatedJson<CreateWithdrawRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data
        .di_container
        .withdraw_service
        .create_withdraw(&body)
        .await
    {
        Ok(response) => Ok((StatusCode::CREATED, Json(json!(response)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    put,
    path = "/api/withdraws/{id}",
    tag = "Withdraw",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Withdrawal ID")
    ),
    request_body = UpdateWithdrawRequest,
    responses(
        (status = 200, description = "Withdrawal record updated successfully", body = ApiResponse<WithdrawResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn update_withdraw(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateWithdrawRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    body.withdraw_id = id;

    match data
        .di_container
        .withdraw_service
        .update_withdraw(&body)
        .await
    {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    delete,
    path = "/api/withdraws/{id}",
    tag = "Withdraw",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Withdrawal ID")
    ),
    responses(
        (status = 200, description = "Withdrawal record deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn delete_withdraw(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.withdraw_service.delete_withdraw(id).await {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Withdraw deleted successfully"
            })),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

pub fn withdraw_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/withdraws", get(get_withdraws))
        .route("/api/withdraw_service/{id}", get(get_withdraw))
        .route("/api/withdraws/users/{id}", get(get_withdraw_users))
        .route("/api/withdraws/user/{id}", get(get_withdraw_user))
        .route("/api/withdraws", post(create_withdraw))
        .route("/api/withdraws/{id}", put(update_withdraw))
        .route("/api/withdraws/{id}", delete(delete_withdraw))
        .route_layer(middleware::from_fn_with_state(app_state.clone(), jwt::auth))
        .with_state(app_state.clone())
}
