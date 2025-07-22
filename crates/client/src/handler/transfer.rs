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
    request::{CreateTransferRequest, FindAllTransferRequest, UpdateTransferRequest},
    response::{ApiResponse, ApiResponsePagination, transfer::TransferResponse},
};
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/transfers",
    tag = "Transfer",
    security(
        ("bearer_auth" = [])
    ),
    params(FindAllTransferRequest),
    responses(
        (status = 200, description = "List of transfer records", body = ApiResponsePagination<Vec<TransferResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn get_transfers(
    State(data): State<Arc<AppState>>,
    Query(params): Query<FindAllTransferRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data
        .di_container
        .transfer_service
        .get_transfers(&params)
        .await
    {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/transfers/{id}",
    tag = "Transfer",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Transfer ID")
    ),
    responses(
        (status = 200, description = "Transfer details retrieved successfully", body = ApiResponse<Option<TransferResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 404, description = "Transfer record not found", body = String),
    )
)]
pub async fn get_transfer(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.transfer_service.get_transfer(id).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/transfers/users/{id}",
    tag = "Transfer",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Transfer details retrieved successfully", body = ApiResponse<Option<Vec<TransferResponse>>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 404, description = "Transfer records not found for the user", body = String),
    )
)]
pub async fn get_transfer_users(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data
        .di_container
        .transfer_service
        .get_transfer_users(id)
        .await
    {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/transfers/user/{id}",
    tag = "Transfer",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Transfer details retrieved successfully", body = ApiResponse<Option<TransferResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn get_transfer_user(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data
        .di_container
        .transfer_service
        .get_transfer_user(id)
        .await
    {
        Ok(saldo) => Ok((StatusCode::OK, Json(json!(saldo)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    post,
    path = "/api/transfers",
    tag = "Transfer",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateTransferRequest,
    responses(
        (status = 201, description = "Transfer record created successfully", body = ApiResponse<TransferResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn create_transfer(
    State(data): State<Arc<AppState>>,
    SimpleValidatedJson(body): SimpleValidatedJson<CreateTransferRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data
        .di_container
        .transfer_service
        .create_transfer(&body)
        .await
    {
        Ok(response) => Ok((StatusCode::CREATED, Json(json!(response)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    put,
    path = "/api/transfers/{id}",
    tag = "Transfer",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Transfer ID")
    ),
    request_body = UpdateTransferRequest,
    responses(
        (status = 200, description = "Transfer record updated successfully", body = ApiResponse<TransferResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn update_transfer(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateTransferRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    body.transfer_id = id;

    match data
        .di_container
        .transfer_service
        .update_transfer(&body)
        .await
    {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    delete,
    path = "/api/transfers/{id}",
    tag = "Transfer",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Transfer ID")
    ),
    responses(
        (status = 200, description = "Transfer record deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn delete_transfer(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.topup_service.delete_topup(id).await {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Transfer deleted successfully"
            })),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

pub fn transfers_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/transfers", get(get_transfers))
        .route("/api/transfers/{id}", get(get_transfer))
        .route("/api/transfers/users/{id}", get(get_transfer_users))
        .route("/api/transfers/user/{id}", get(get_transfer_user))
        .route("/api/transfers", post(create_transfer))
        .route("/api/transfers/{id}", put(update_transfer))
        .route("/api/transfers/{id}", delete(delete_transfer))
        .route_layer(middleware::from_fn_with_state(app_state.clone(), jwt::auth))
        .with_state(app_state.clone())
}
