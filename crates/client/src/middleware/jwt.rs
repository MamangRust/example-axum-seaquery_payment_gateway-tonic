use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::IntoResponse,
};
use axum_extra::extract::cookie::CookieJar;
use shared::domain::response::ErrorResponse;
use std::sync::Arc;

use crate::state::AppState;

pub async fn auth(
    cookie_jar: CookieJar,
    State(data): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let token = cookie_jar
        .get("token")
        .map(|cookie| cookie.value().to_string())
        .or_else(|| {
            req.headers()
                .get(header::AUTHORIZATION)
                .and_then(|auth_header| auth_header.to_str().ok())
                .and_then(|auth_value| auth_value.strip_prefix("Bearer ").map(str::to_owned))
        });

    let token = match token {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    status: "fail".to_string(),
                    message: "You are not logged in, please provide token".to_string(),
                }),
            ));
        }
    };

    let user_id = match data.jwt_config.verify_token(&token) {
        Ok(id) => id as i32,
        Err(_) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    status: "fail".to_string(),
                    message: "Invalid token".to_string(),
                }),
            ));
        }
    };

    req.extensions_mut().insert(user_id);

    Ok(next.run(req).await)
}
