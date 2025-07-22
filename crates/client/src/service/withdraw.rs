use async_trait::async_trait;
use genproto::withdraw::{
    CreateWithdrawRequest, FindAllWithdrawRequest, FindWithdrawByIdRequest,
    FindWithdrawByUserIdRequest, UpdateWithdrawRequest,
    withdraw_service_client::WithdrawServiceClient,
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
            CreateWithdrawRequest as DomainCreateWithdrawRequest,
            FindAllWithdrawRequest as DomainFindAllWithdrawRequest,
            UpdateWithdrawRequest as DomainUpdateWithdrawRequest,
        },
        response::{ApiResponse, ApiResponsePagination, ErrorResponse, withdraw::WithdrawResponse},
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

use shared::abstract_trait::WithdrawServiceTrait;

#[derive(Debug)]
pub struct WithdrawService {
    client: Arc<Mutex<WithdrawServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl WithdrawService {
    pub async fn new(
        client: Arc<Mutex<WithdrawServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Self {
        registry.register(
            "withdraw_handler_request_counter",
            "total number of requests to the WithdrawService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "withdraw_handler_request_duration",
            "Histogram of request durations for the WithdrawService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("withdraw-service-client")
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
impl WithdrawServiceTrait for WithdrawService {
    async fn get_withdraws(
        &self,
        req: &DomainFindAllWithdrawRequest,
    ) -> Result<ApiResponsePagination<Vec<WithdrawResponse>>, ErrorResponse> {
        info!(
            "Getting all withdraws (page: {}, size: {}, search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "GetAllWithdraws",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("operation", "get_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllWithdrawRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_all_withdraw(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponsePagination {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into_iter().map(Into::into).collect(),
                    pagination: inner.pagination.unwrap_or_default().into(),
                };

                info!(
                    "Withdraws retrieved successfully (page: {}, size: {})",
                    req.page, req.page_size
                );

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Withdraws retrieved successfully (page: {}, size: {})",
                        req.page, req.page_size
                    ),
                )
                .await;

                Ok(response)
            }
            Err(err) => {
                let error_response = ErrorResponse {
                    status: err.code().to_string(),
                    message: err.message().to_string(),
                };

                error!(
                    "Failed to retrieve withdraws (page: {}, size: {}): {}",
                    req.page, req.page_size, error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve withdraws (page: {}, size: {}): {}",
                        req.page, req.page_size, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_withdraw(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<WithdrawResponse>>, ErrorResponse> {
        info!("Getting withdraw {id}");

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "GetWithdrawById",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("operation", "get_by_id"),
                KeyValue::new("withdraw.id", id as i64),
            ],
        );

        let mut request = Request::new(FindWithdrawByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_withdraw_by_id(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.map(Into::into),
                };

                info!("Withdraw {id} found successfully");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Withdraw {id} found successfully"),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                error!("Failed to find withdraw {id}: {}", error_response.message);

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to find withdraw {id}: {}", error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_withdraw_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<WithdrawResponse>>>, ErrorResponse> {
        info!("Getting withdraws for user {id}");

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindWithdrawByUserId",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("operation", "get_withdraw_users"),
                KeyValue::new("user.id", id as i64),
            ],
        );

        let mut request = Request::new(FindWithdrawByUserIdRequest { user_id: id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self
            .client
            .lock()
            .await
            .find_withdraw_by_users_id(request)
            .await
        {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: Some(inner.data.into_iter().map(Into::into).collect()),
                };

                info!("Withdraws for user {id} retrieved successfully");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Withdraws for user {id} retrieved successfully"),
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
                    "Failed to retrieve withdraws for user {id}: {}",
                    error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve withdraws for user {id}: {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_withdraw_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<WithdrawResponse>>, ErrorResponse> {
        info!("Getting withdraw for user {id}");

        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "GetWithdrawUser",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("operation", "get_withdraw_user"),
                KeyValue::new("user.id", id as i64),
            ],
        );

        let mut request = Request::new(FindWithdrawByUserIdRequest { user_id: id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self
            .client
            .lock()
            .await
            .find_withdraw_by_user_id(request)
            .await
        {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.map(Into::into),
                };

                info!("Withdraw for user {id} found successfully");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Withdraw for user {id} found successfully"),
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
                    "Failed to find withdraw for user {id}: {}",
                    error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to find withdraw for user {id}: {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn create_withdraw(
        &self,
        input: &DomainCreateWithdrawRequest,
    ) -> Result<ApiResponse<WithdrawResponse>, ErrorResponse> {
        info!("Creating withdraw for user_id {}", input.user_id);

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateWithdraw",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("operation", "create"),
                KeyValue::new("withdraw.user_id", input.user_id as i64),
                KeyValue::new("withdraw.amount", input.withdraw_amount as i64),
                KeyValue::new("withdraw.time", input.withdraw_time.timestamp()),
            ],
        );

        let mut request = Request::new(CreateWithdrawRequest {
            user_id: input.user_id,
            withdraw_amount: input.withdraw_amount,
            withdraw_time: input.withdraw_time.to_string(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.create_withdraw(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                info!(
                    "Withdraw for user_id {} created successfully",
                    input.user_id
                );

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Withdraw for user_id {} created successfully",
                        input.user_id
                    ),
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
                    "Failed to create withdraw for user_id {}: {}",
                    input.user_id, error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to create withdraw for user_id {}: {}",
                        input.user_id, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn update_withdraw(
        &self,
        input: &DomainUpdateWithdrawRequest,
    ) -> Result<ApiResponse<WithdrawResponse>, ErrorResponse> {
        info!("Updating withdraw for withdraw_id {}", input.withdraw_id);

        let method = Method::Put;
        let withdraw_id = input.withdraw_id;
        let user_id = input.user_id;
        let withdraw_amount = input.withdraw_amount;
        let withdraw_time = input.withdraw_time;

        let tracing_ctx = self.start_tracing(
            "UpdateWithdraw",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("operation", "update"),
                KeyValue::new("withdraw.id", withdraw_id as i64),
                KeyValue::new("withdraw.user_id", user_id as i64),
                KeyValue::new("withdraw.amount", withdraw_amount as i64),
                KeyValue::new("withdraw.time", withdraw_time.timestamp()),
            ],
        );

        let update_request = UpdateWithdrawRequest {
            withdraw_id,
            user_id,
            withdraw_amount,
            withdraw_time: withdraw_time.to_string(),
        };

        let mut request = Request::new(update_request);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.update_withdraw(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                info!("Withdraw updated successfully (ID: {withdraw_id}, user_id: {user_id})");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Withdraw updated successfully (ID: {withdraw_id}, user_id: {user_id})"
                    ),
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
                    "Failed to update withdraw (ID: {withdraw_id}, user_id: {user_id}): {}",
                    error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to update withdraw (ID: {withdraw_id}, user_id: {user_id}): {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn delete_withdraw(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        info!("Deleting withdraw {id}");

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteWithdraw",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("withdraw.id", id as i64),
            ],
        );

        let mut request = Request::new(FindWithdrawByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.delete_withdraw(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: (),
                };

                info!("Withdraw {id} deleted successfully");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Withdraw {id} deleted successfully"),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                error!("Failed to delete withdraw {id}: {}", error_response.message);

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to delete withdraw {id}: {}", error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }
}
