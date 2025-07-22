use async_trait::async_trait;
use genproto::saldo::{
    CreateSaldoRequest, FindAllSaldoRequest, FindSaldoByIdRequest, FindSaldoByUserIdRequest,
    UpdateSaldoRequest, saldo_service_client::SaldoServiceClient,
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
            CreateSaldoRequest as DomainCreateSaldoRequest,
            FindAllSaldoRequest as DomainFindAllSaldoRequest,
            UpdateSaldoRequest as DomainUpdateSaldoRequest,
        },
        response::{ApiResponse, ApiResponsePagination, ErrorResponse, saldo::SaldoResponse},
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

use shared::abstract_trait::SaldoServiceTrait;

#[derive(Debug)]
pub struct SaldoService {
    client: Arc<Mutex<SaldoServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl SaldoService {
    pub async fn new(
        client: Arc<Mutex<SaldoServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Self {
        registry.register(
            "saldo_handler_request_counter",
            "Total number of requests to the SaldoService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "saldo_handler_request_duration",
            "Histogram of request durations for the SaldoService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("saldo-service-client")
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
impl SaldoServiceTrait for SaldoService {
    async fn get_saldos(
        &self,
        req: &DomainFindAllSaldoRequest,
    ) -> Result<ApiResponsePagination<Vec<SaldoResponse>>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "GetAllSaldos",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("operation", "get_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllSaldoRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_all_saldo(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponsePagination {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.into_iter().map(Into::into).collect(),
                    pagination: inner.pagination.unwrap_or_default().into(),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Saldos retrieved successfully (page: {}, size: {})",
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

    async fn get_saldo(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<SaldoResponse>>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "GetSaldoById",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("operation", "get_by_id"),
                KeyValue::new("saldo.id", id as i64),
            ],
        );

        let mut request = Request::new(FindSaldoByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_saldo_by_id(request).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.map(Into::into),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Saldo {id} found successfully"),
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
                    &format!("Failed to find saldo {id}: {}", error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_saldo_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<SaldoResponse>>>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "FindSaldoByUserId",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("operation", "get_saldo_users"),
                KeyValue::new("user.id", id as i64),
            ],
        );

        let mut request = Request::new(FindSaldoByUserIdRequest { user_id: id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self
            .client
            .lock()
            .await
            .find_saldo_by_users_id(request)
            .await
        {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: Some(inner.data.into_iter().map(Into::into).collect()),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Saldo for user {id} retrieved successfully"),
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
                        "Failed to retrieve saldo for user {id}: {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_saldo_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<SaldoResponse>>, ErrorResponse> {
        let method = Method::Get;
        let tracing_ctx = self.start_tracing(
            "GetSaldoUser",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("operation", "get_saldo_user"),
                KeyValue::new("user.id", id as i64),
            ],
        );

        let mut request = Request::new(FindSaldoByUserIdRequest { user_id: id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self
            .client
            .lock()
            .await
            .find_saldo_by_user_id(request)
            .await
        {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: inner.data.map(Into::into),
                };

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    &format!("Saldo for user {id} found successfully"),
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
                        "Failed to find saldo for user {id}: {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn create_saldo(
        &self,
        input: &DomainCreateSaldoRequest,
    ) -> Result<ApiResponse<SaldoResponse>, ErrorResponse> {
        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateSaldo",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("operation", "create"),
                KeyValue::new("saldo.user_id", input.user_id as i64),
                KeyValue::new("saldo.total_balance", input.total_balance as i64),
            ],
        );

        let mut request = Request::new(CreateSaldoRequest {
            user_id: input.user_id,
            total_balance: input.total_balance,
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.create_saldo(request).await {
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
                    &format!("Saldo for user_id {} created successfully", input.user_id),
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
                        "Failed to create saldo for user_id {}: {}",
                        input.user_id, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn update_saldo(
        &self,
        input: &DomainUpdateSaldoRequest,
    ) -> Result<ApiResponse<SaldoResponse>, ErrorResponse> {
        let method = Method::Put;
        let saldo_id = input.saldo_id;
        let user_id = input.user_id;
        let total_balance = input.total_balance;

        let tracing_ctx = self.start_tracing(
            "UpdateSaldo",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("operation", "update"),
                KeyValue::new("saldo.id", saldo_id as i64),
                KeyValue::new("saldo.user_id", user_id as i64),
                KeyValue::new("saldo.total_balance", total_balance as i64),
            ],
        );

        let update_request = UpdateSaldoRequest {
            saldo_id,
            user_id,
            total_balance,
        };

        let mut request = Request::new(update_request);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.update_saldo(request).await {
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
                    &format!("Saldo updated successfully (ID: {saldo_id}, user_id: {user_id})"),
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
                        "Failed to update saldo (ID: {saldo_id}, user_id: {user_id}): {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn delete_saldo(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteSaldo",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("saldo.id", id as i64),
            ],
        );

        let mut request = Request::new(FindSaldoByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.delete_saldo(request).await {
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
                    &format!("Saldo {id} deleted successfully"),
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
                    &format!("Failed to delete saldo {}: {}", id, error_response.message),
                )
                .await;

                Err(error_response)
            }
        }
    }
}
