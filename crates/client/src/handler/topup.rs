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
    request::{CreateTopupRequest, FindAllTopupRequest, UpdateTopupRequest},
    response::{ApiResponse, ApiResponsePagination, topup::TopupResponse},
};
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/topups",
    tag = "Topup",
    security(
        ("bearer_auth" = [])
    ),
    params(FindAllTopupRequest),
    responses(
        (status = 200, description = "List of topup records", body = ApiResponsePagination<Vec<TopupResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn get_topups(
    State(data): State<Arc<AppState>>,
    Query(params): Query<FindAllTopupRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.topup_service.get_topups(&params).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/topups/{id}",
    tag = "Topup",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Topup ID")
    ),
    responses(
        (status = 200, description = "Topup details retrieved successfully", body = ApiResponse<Option<TopupResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 404, description = "Topup record not found", body = String),
    )
)]
pub async fn get_topup(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.topup_service.get_topup(id).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/topups/users/{id}",
    tag = "Topup",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Topup details retrieved successfully", body = ApiResponse<Option<Vec<TopupResponse>>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 404, description = "Topup records not found for the user", body = String),
    )
)]
pub async fn get_topup_users(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.topup_service.get_topup_users(id).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/topups/user/{id}",
    tag = "Topup",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Topup details retrieved successfully", body = ApiResponse<Option<TopupResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn get_topup_user(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.topup_service.get_topup_user(id).await {
        Ok(saldo) => Ok((StatusCode::OK, Json(json!(saldo)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    post,
    path = "/api/topups",
    tag = "Topup",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateTopupRequest,
    responses(
        (status = 201, description = "Topup record created successfully", body = ApiResponse<TopupResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn create_topup(
    State(data): State<Arc<AppState>>,
    SimpleValidatedJson(body): SimpleValidatedJson<CreateTopupRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.topup_service.create_topup(&body).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(json!(response)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    put,
    path = "/api/topups/{id}",
    tag = "Topup",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Topup ID")
    ),
    request_body = UpdateTopupRequest,
    responses(
        (status = 200, description = "Topup record updated successfully", body = ApiResponse<TopupResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn update_topup(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateTopupRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    body.topup_id = id;

    match data.di_container.topup_service.update_topup(&body).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    delete,
    path = "/api/topups/{id}",
    tag = "Topup",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Topup ID")
    ),
    responses(
        (status = 200, description = "Topup record deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn delete_topup(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.topup_service.delete_topup(id).await {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Topup deleted successfully"
            })),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

pub fn topup_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/topups", get(get_topups))
        .route("/api/topups/{id}", get(get_topup))
        .route("/api/topups/users/{id}", get(get_topup_users))
        .route("/api/topups/user/{id}", get(get_topup_user))
        .route("/api/topups", post(create_topup))
        .route("/api/topups/{id}", put(update_topup))
        .route("/api/topups/{id}", delete(delete_topup))
        .route_layer(middleware::from_fn_with_state(app_state.clone(), jwt::auth))
        .with_state(app_state.clone())
}
