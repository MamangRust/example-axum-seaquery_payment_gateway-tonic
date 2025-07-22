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
    request::{CreateSaldoRequest, FindAllSaldoRequest, UpdateSaldoRequest},
    response::{ApiResponse, ApiResponsePagination, saldo::SaldoResponse},
};
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;

#[utoipa::path(
    get,
    path = "/api/saldos",
    tag = "Saldo",
    security(
        ("bearer_auth" = [])
    ),
    params(FindAllSaldoRequest),
    responses(
        (status = 200, description = "List of saldo records", body = ApiResponsePagination<Vec<SaldoResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn get_saldos(
    State(data): State<Arc<AppState>>,
    Query(params): Query<FindAllSaldoRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.saldo_service.get_saldos(&params).await {
        Ok(saldoes) => Ok((StatusCode::OK, Json(json!(saldoes)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/saldos/{id}",
    tag = "Saldo",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Saldo ID")
    ),
    responses(
        (status = 200, description = "Saldo details retrieved successfully", body = ApiResponse<Option<SaldoResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 404, description = "Saldo record not found", body = String),
    )
)]
pub async fn get_saldo(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.saldo_service.get_saldo(id).await {
        Ok(saldo) => Ok((StatusCode::OK, Json(json!(saldo)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/saldos/users/{id}",
    tag = "Saldo",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Saldo details retrieved successfully", body = ApiResponse<Option<Vec<SaldoResponse>>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 404, description = "Saldo records not found for the user", body = String),
    )
)]
pub async fn get_saldo_users(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.saldo_service.get_saldo_users(id).await {
        Ok(saldo) => Ok((StatusCode::OK, Json(json!(saldo)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    get,
    path = "/api/saldos/user/{id}",
    tag = "Saldo",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Saldo details retrieved successfully", body = ApiResponse<Option<SaldoResponse>>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn get_saldo_user(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.saldo_service.get_saldo_user(id).await {
        Ok(saldo) => Ok((StatusCode::OK, Json(json!(saldo)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    post,
    path = "/api/saldos",
    tag = "Saldo",
    request_body = CreateSaldoRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 201, description = "Saldo record created successfully", body = ApiResponse<SaldoResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn create_saldo(
    State(data): State<Arc<AppState>>,
    SimpleValidatedJson(body): SimpleValidatedJson<CreateSaldoRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.saldo_service.create_saldo(&body).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(json!(response)))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    put,
    path = "/api/saldos/{id}",
    tag = "Saldo",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Saldo ID")
    ),
    request_body = UpdateSaldoRequest,
    responses(
        (status = 200, description = "Saldo record updated successfully", body = ApiResponse<SaldoResponse>),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn update_saldo(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    SimpleValidatedJson(mut body): SimpleValidatedJson<UpdateSaldoRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    body.saldo_id = id;

    match data.di_container.saldo_service.update_saldo(&body).await {
        Ok(response) => Ok((StatusCode::OK, Json(json!(response)))),

        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

#[utoipa::path(
    delete,
    path = "/api/saldos/{id}",
    tag = "Saldo",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = i32, Path, description = "Saldo ID")
    ),
    responses(
        (status = 200, description = "Saldo record deleted successfully", body = serde_json::Value),
        (status = 401, description = "Unauthorized access", body = String),
        (status = 500, description = "Internal server error", body = String),
    )
)]
pub async fn delete_saldo(
    State(data): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Extension(_user_id): Extension<i64>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match data.di_container.saldo_service.delete_saldo(id).await {
        Ok(_) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Saldo deleted successfully"
            })),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!(e)))),
    }
}

pub fn saldos_routes(app_state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .route("/api/saldos", get(get_saldos))
        .route("/api/saldos/{id}", get(get_saldo))
        .route("/api/saldos/users/{id}", get(get_saldo_users))
        .route("/api/saldos/user/{id}", get(get_saldo_user))
        .route("/api/saldos", post(create_saldo))
        .route("/api/saldos/{id}", put(update_saldo))
        .route("/api/saldos/{id}", delete(delete_saldo))
        .route_layer(middleware::from_fn_with_state(app_state.clone(), jwt::auth))
        .with_state(app_state.clone())
}
