use async_trait::async_trait;
use genproto::user::{
    CreateUserRequest, FindAllUserRequest, FindUserByIdRequest, UpdateUserRequest,
    user_service_client::UserServiceClient,
};
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;

use shared::{
    domain::{
        request::{
            FindAllUserRequest as DomainFindAllUserRequest,
            RegisterRequest as DomainCreateUserRequest,
            UpdateUserRequest as DomainUpdateUserRequest,
        },
        response::{ApiResponse, ApiResponsePagination, ErrorResponse, user::UserResponse},
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

use shared::abstract_trait::UserServiceTrait;

#[derive(Debug)]
pub struct UserService {
    client: Arc<Mutex<UserServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl UserService {
    pub async fn new(
        client: Arc<Mutex<UserServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Self {
        registry.register(
            "User_handler_request_counter",
            "total number of requests to the UserService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "User_handler_request_duration",
            "Histogram of request durations for the UserService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("User-service-client")
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
impl UserServiceTrait for UserService {
    async fn get_users(
        &self,
        req: &DomainFindAllUserRequest,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetAllUsers",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "get_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllUserRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_all_users(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponsePagination {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into_iter().map(Into::into).collect(),
                    pagination: inner.pagination.unwrap_or_default().into(),
                };
                self.complete_tracing_success(&tracing_ctx, method, &response.message)
                    .await;
                Ok(response)
            }
            Err(err) => {
                let error_response = ErrorResponse {
                    status: err.code().to_string(),
                    message: err.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve saldos (page: {}, size: {}): {}",
                        req.page, req.page_size, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_user(&self, id: i32) -> Result<ApiResponse<Option<UserResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindUserByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_by_id(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.map(Into::into),
                };

                self.complete_tracing_success(&tracing_ctx, method, &response.message)
                    .await;
                Ok(response)
            }
            Err(err) => {
                let error_response = ErrorResponse {
                    status: err.code().to_string(),
                    message: err.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve user (id: {}): {}",
                        id, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn create_user(
        &self,
        input: &DomainCreateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "CreateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "create"),
                KeyValue::new("user.email", input.email.clone()),
            ],
        );

        let mut request = Request::new(CreateUserRequest {
            firstname: input.firstname.clone(),
            lastname: input.lastname.clone(),
            email: input.email.clone(),
            password: input.password.clone(),
            confirm_password: input.confirm_password.clone(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.create_user(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("User {} registered successfully", input.email),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to register user {}: {}",
                        input.email, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn update_user(
        &self,
        input: &DomainUpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        let method = Method::Put;
        let user_id = input.id;

        let tracing_ctx = self.start_tracing(
            "UpdateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "update"),
                KeyValue::new("user.id", user_id as i64),
            ],
        );

        let update_request = UpdateUserRequest {
            id: user_id,
            firstname: input.firstname.clone().unwrap_or_default(),
            lastname: input.lastname.clone().unwrap_or_default(),
            email: input.email.clone().unwrap_or_default(),
            password: input.password.clone(),
            confirm_password: input.confirm_password.clone(),
        };

        let mut request = Request::new(update_request);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.update_user(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("User updated successfully (ID: {user_id})"),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to update user (ID: {user_id}): {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn delete_user(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("user.id", id as i64),
            ],
        );

        let mut request = Request::new(FindUserByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.delete_user(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: (),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("User {id} deleted successfully"),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to delete user {}: {}", id, error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }
}
