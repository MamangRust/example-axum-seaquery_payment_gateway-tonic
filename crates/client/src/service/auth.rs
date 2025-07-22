use async_trait::async_trait;
use genproto::auth::{
    GetMeRequest, LoginRequest, RegisterRequest, auth_service_client::AuthServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use shared::{
    abstract_trait::AuthServiceTrait,
    domain::{
        request::{LoginRequest as LoginDomainRequest, RegisterRequest as RegisterDomainRequest},
        response::{ApiResponse, ErrorResponse, user::UserResponse},
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

#[derive(Debug)]
pub struct AuthService {
    client: Arc<Mutex<AuthServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl AuthService {
    pub async fn new(
        client: Arc<Mutex<AuthServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Self {
        registry.register(
            "auth_handler_request_counter",
            "Total number of requests to the AuthService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "auth_handler_request_duration",
            "Histogram of request durations for the AuthService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("auth-service-client")
    }

    fn inject_trace_context<T>(&self, cx: &Context, request: &mut Request<T>) {
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(cx, &mut MetadataInjector(request.metadata_mut()))
        });
    }

    fn start_tracing(&self, operation_name: &str, attributes: Vec<KeyValue>) -> TracingContext {
        let start_time = Instant::now();
        let tracer = self.get_tracer();
        let mut span = tracer
            .span_builder(operation_name.to_string())
            .with_kind(SpanKind::Server)
            .with_attributes(attributes)
            .start(&tracer);

        info!("Starting operation: {operation_name}");

        span.add_event(
            "Operation started",
            vec![
                KeyValue::new("operation", operation_name.to_string()),
                KeyValue::new("timestamp", start_time.elapsed().as_secs_f64().to_string()),
            ],
        );

        let cx = Context::current_with_span(span);
        TracingContext { cx, start_time }
    }

    async fn complete_tracing_success(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        message: &str,
    ) {
        self.complete_tracing_internal(tracing_ctx, method, true, message)
            .await;
    }

    async fn complete_tracing_error(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        error_message: &str,
    ) {
        self.complete_tracing_internal(tracing_ctx, method, false, error_message)
            .await;
    }

    async fn complete_tracing_internal(
        &self,
        tracing_ctx: &TracingContext,
        method: Method,
        is_success: bool,
        message: &str,
    ) {
        let status_str = if is_success { "SUCCESS" } else { "ERROR" };
        let status = if is_success {
            StatusUtils::Success
        } else {
            StatusUtils::Error
        };
        let elapsed = tracing_ctx.start_time.elapsed().as_secs_f64();

        tracing_ctx.cx.span().add_event(
            "Operation completed",
            vec![
                KeyValue::new("status", status_str),
                KeyValue::new("duration_secs", elapsed.to_string()),
                KeyValue::new("message", message.to_string()),
            ],
        );

        if is_success {
            info!("Operation completed successfully: {message}");
        } else {
            error!("Operation failed: {message}");
        }

        self.metrics.lock().await.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl AuthServiceTrait for AuthService {
    async fn register_user(
        &self,
        request_data: &RegisterDomainRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        info!("Registering user: {}", request_data.email);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RegisterUser",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("operation", "register"),
                KeyValue::new("user.email", request_data.email.clone()),
                KeyValue::new("user.firstname", request_data.firstname.clone()),
            ],
        );

        let mut request = Request::new(RegisterRequest {
            firstname: request_data.firstname.clone(),
            lastname: request_data.lastname.clone(),
            email: request_data.email.clone(),
            password: request_data.password.clone(),
            confirm_password: request_data.confirm_password.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let result = {
            let mut client = self.client.lock().await;
            client.register_user(request).await
        };

        match result {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                info!("User {} registered successfully", request_data.email);

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("User {} registered successfully", request_data.email),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                error!(
                    "Failed to register user {}: {}",
                    request_data.email, error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to register user {}: {}",
                        request_data.email, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn login_user(
        &self,
        request_data: &LoginDomainRequest,
    ) -> Result<ApiResponse<String>, ErrorResponse> {
        info!("Logging in user: {}", request_data.email);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "LoginUser",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("operation", "login"),
                KeyValue::new("user.email", request_data.email.clone()),
            ],
        );

        let mut request = Request::new(LoginRequest {
            email: request_data.email.clone(),
            password: request_data.password.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let result = {
            let mut client = self.client.lock().await;
            client.login_user(request).await
        };

        match result {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data,
                };

                info!("User {} logged in successfully", request_data.email);

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("User {} logged in successfully", request_data.email),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                error!(
                    "Failed to login user {}: {}",
                    request_data.email, error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to login user {}: {}",
                        request_data.email, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_me(&self, id: i32) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetMe",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.id", id.to_string()),
            ],
        );

        let mut request = Request::new(GetMeRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let result = {
            let mut client = self.client.lock().await;
            client.get_me(request).await
        };

        match result {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                info!("User profile {id} retrieved successfully");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("User profile {id} retrieved successfully"),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                error!(
                    "Failed to retrieve user profile {id}: {}",
                    error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve user profile {id}: {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }
}
