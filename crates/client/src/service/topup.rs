use async_trait::async_trait;
use genproto::topup::{
    CreateTopupRequest, FindAllTopupRequest, FindTopupByIdRequest, FindTopupByUserIdRequest,
    UpdateTopupRequest, topup_service_client::TopupServiceClient,
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
            CreateTopupRequest as DomainCreateTopupRequest,
            FindAllTopupRequest as DomainFindAllTopupRequest,
            UpdateTopupRequest as DomainUpdateTopupRequest,
        },
        response::{ApiResponse, ApiResponsePagination, ErrorResponse, topup::TopupResponse},
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

use shared::abstract_trait::TopupServiceTrait;

#[derive(Debug)]
pub struct TopupService {
    client: Arc<Mutex<TopupServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl TopupService {
    pub async fn new(
        client: Arc<Mutex<TopupServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Self {
        registry.register(
            "topup_handler_request_counter",
            "total number of requests to the TopupService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "topup_handler_request_duration",
            "Histogram of request durations for the TopupService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("topup-service-client")
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
impl TopupServiceTrait for TopupService {
    async fn get_topups(
        &self,
        req: &DomainFindAllTopupRequest,
    ) -> Result<ApiResponsePagination<Vec<TopupResponse>>, ErrorResponse> {
        info!(
            "Retrieving topups (page: {}, size: {}, search: {})",
            req.page, req.page_size, req.search
        );

        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetAllTopups",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("operation", "get_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllTopupRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_all_topup(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponsePagination {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into_iter().map(Into::into).collect(),
                    pagination: inner.pagination.unwrap_or_default().into(),
                };

                info!(
                    "Retrieved topups (page: {}, size: {}): {}",
                    req.page, req.page_size, response.message
                );

                self.complete_tracing_success(&tracing_ctx, method, &response.message)
                    .await;
                Ok(response)
            }
            Err(err) => {
                let error_response = ErrorResponse {
                    status: err.code().to_string(),
                    message: err.message().to_string(),
                };

                error!(
                    "Failed to retrieve topups (page: {}, size: {}): {}",
                    req.page, req.page_size, error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve topups (page: {}, size: {}): {}",
                        req.page, req.page_size, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_topup(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<TopupResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTopup",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("operation", "get"),
                KeyValue::new("id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindTopupByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_topup_by_id(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.map(Into::into),
                };

                info!("Retrieved topup (id: {}): {}", id, response.message);

                self.complete_tracing_success(&tracing_ctx, method, &response.message)
                    .await;
                Ok(response)
            }
            Err(err) => {
                let error_response = ErrorResponse {
                    status: err.code().to_string(),
                    message: err.message().to_string(),
                };

                error!(
                    "Failed to retrieve topup (id: {}): {id}",
                    error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve topup (id: {id}): {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_topup_users(
        &self,
        user_id: i32,
    ) -> Result<ApiResponse<Option<Vec<TopupResponse>>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTopupUsers",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("operation", "get_users"),
                KeyValue::new("user_id", user_id.to_string()),
            ],
        );

        let mut request = Request::new(FindTopupByUserIdRequest { user_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self
            .client
            .lock()
            .await
            .find_topup_by_users_id(request)
            .await
        {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: Some(inner.data.into_iter().map(Into::into).collect()),
                };

                info!(
                    "Retrieved topups (user_id: {user_id}): {}",
                    response.message
                );

                self.complete_tracing_success(&tracing_ctx, method, &response.message)
                    .await;
                Ok(response)
            }
            Err(err) => {
                let error_response = ErrorResponse {
                    status: err.code().to_string(),
                    message: err.message().to_string(),
                };

                error!(
                    "Failed to retrieve topup (user_id: {user_id}): {}",
                    error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve topup (user_id: {user_id}): {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_topup_user(
        &self,
        user_id: i32,
    ) -> Result<ApiResponse<Option<TopupResponse>>, ErrorResponse> {
        info!("Retrieving topup (user_id: {user_id})");

        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTopupUser",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("operation", "get_user"),
                KeyValue::new("user_id", user_id.to_string()),
            ],
        );

        let mut request = Request::new(FindTopupByUserIdRequest { user_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self
            .client
            .lock()
            .await
            .find_topup_by_user_id(request)
            .await
        {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.map(Into::into),
                };

                info!("Retrieved topup (user_id: {user_id}): {}", response.message);

                self.complete_tracing_success(&tracing_ctx, method, &response.message)
                    .await;
                Ok(response)
            }
            Err(err) => {
                let error_response = ErrorResponse {
                    status: err.code().to_string(),
                    message: err.message().to_string(),
                };

                error!(
                    "Failed to retrieve topup (user_id: {user_id}): {}",
                    error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to retrieve topup (user_id: {user_id}): {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn create_topup(
        &self,
        input: &DomainCreateTopupRequest,
    ) -> Result<ApiResponse<TopupResponse>, ErrorResponse> {
        info!(
            "Creating topup (user_id: {}, topup_no: {}, topup_amount: {}, topup_method: {})",
            input.user_id, input.topup_no, input.topup_amount, input.topup_method
        );

        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateTopup",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("operation", "create"),
                KeyValue::new("topup.user_id", input.user_id as i64),
                KeyValue::new("topup.topup_no", input.topup_no.clone()),
                KeyValue::new("topup.topup_amount", input.topup_amount as i64),
                KeyValue::new("topup.topup_method", input.topup_method.clone()),
            ],
        );

        let mut request = Request::new(CreateTopupRequest {
            user_id: input.user_id,
            topup_no: input.topup_no.clone(),
            topup_amount: input.topup_amount,
            topup_method: input.topup_method.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.create_topup(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                info!(
                    "Topup {} for user_id {} created successfully",
                    input.topup_no, input.user_id
                );

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Topup {} for user_id {} created successfully",
                        input.topup_no, input.user_id
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
                    "Failed to create topup {} for user_id {}: {}",
                    input.topup_no, input.user_id, error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to create topup {} for user_id {}: {}",
                        input.topup_no, input.user_id, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn update_topup(
        &self,
        input: &DomainUpdateTopupRequest,
    ) -> Result<ApiResponse<TopupResponse>, ErrorResponse> {
        info!(
            "Updating topup (topup_id: {}, user_id: {}, topup_amount: {}, topup_method: {})",
            input.topup_id, input.user_id, input.topup_amount, input.topup_method
        );

        let method = Method::Put;
        let topup_id = input.topup_id;
        let user_id = input.user_id;
        let topup_amount = input.topup_amount;
        let topup_method = input.topup_method.clone();

        let tracing_ctx = self.start_tracing(
            "UpdateTopup",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("operation", "update"),
                KeyValue::new("topup.id", topup_id as i64),
                KeyValue::new("topup.user_id", user_id as i64),
                KeyValue::new("topup.amount", topup_amount as i64),
                KeyValue::new("topup.method", topup_method.clone()),
            ],
        );

        let update_request = UpdateTopupRequest {
            topup_id,
            user_id,
            topup_amount,
            topup_method,
        };

        let mut request = Request::new(update_request);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.update_topup(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into(),
                };

                info!("Topup updated successfully (ID: {topup_id}, user_id: {user_id})");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Topup updated successfully (ID: {topup_id}, user_id: {user_id})"),
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
                    "Failed to update topup (ID: {topup_id}, user_id: {user_id}): {}",
                    error_response.message
                );

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to update topup (ID: {topup_id}, user_id: {user_id}): {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn delete_topup(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        info!("Deleting topup (id: {})", id);

        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteTopup",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("topup.id", id as i64),
            ],
        );

        let mut request = Request::new(FindTopupByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.delete_topup(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: (),
                };

                info!("Topup {id} deleted successfully");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Topup {id} deleted successfully"),
                )
                .await;

                Ok(response)
            }
            Err(status) => {
                let error_response = ErrorResponse {
                    status: status.code().to_string(),
                    message: status.message().to_string(),
                };

                error!("Failed to delete topup {}: {}", id, error_response.message);

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to delete topup {}: {}", id, error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }
}
