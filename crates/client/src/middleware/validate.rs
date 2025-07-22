use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use validator::{Validate, ValidationErrors};

pub struct SimpleValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for SimpleValidatedJson<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = (StatusCode, axum::Json<Value>);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(json_value) =
            axum::Json::<T>::from_request(req, state)
                .await
                .map_err(|rejection| {
                    let payload = json!({
                        "error": "Invalid JSON",
                        "message": rejection.body_text(),
                    });
                    (rejection.status(), axum::Json(payload))
                })?;

        json_value.validate().map_err(|validation_errors| {
            let payload = json!({
                "error": "Validation failed",
                "message": format_validation_errors(&validation_errors),
                "details": format_validation_errors_detailed(&validation_errors)
            });
            (StatusCode::BAD_REQUEST, axum::Json(payload))
        })?;

        Ok(Self(json_value))
    }
}

fn format_validation_errors(errors: &ValidationErrors) -> String {
    let mut error_messages = Vec::new();

    for (field, field_errors) in errors.field_errors() {
        for error in field_errors {
            let message = error
                .message
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_else(|| match error.code.as_ref() {
                    "email" => "Invalid email format".to_string(),
                    "url" => "Invalid URL format".to_string(),
                    "length" => "Invalid length".to_string(),
                    "range" => "Value out of range".to_string(),
                    "custom" => "Custom validation failed".to_string(),
                    _ => format!("Invalid {field}"),
                });
            error_messages.push(format!("{field}: {message}"));
        }
    }

    if error_messages.is_empty() {
        "Validation failed".to_string()
    } else {
        error_messages.join("; ")
    }
}

fn format_validation_errors_detailed(errors: &ValidationErrors) -> Value {
    let mut error_map = serde_json::Map::new();

    for (field, field_errors) in errors.field_errors() {
        let messages: Vec<String> = field_errors
            .iter()
            .map(|e| {
                e.message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| match e.code.as_ref() {
                        "email" => "Invalid email format".to_string(),
                        "url" => "Invalid URL format".to_string(),
                        "length" => "Invalid length".to_string(),
                        "range" => "Value out of range".to_string(),
                        "custom" => "Custom validation failed".to_string(),
                        _ => format!("Invalid {field}"),
                    })
            })
            .collect();
        error_map.insert(field.to_string(), json!(messages));
    }

    json!(error_map)
}
