use async_trait::async_trait;
use genproto::transfer::{
    CreateTransferRequest, FindAllTransferRequest, FindTransferByIdRequest,
    FindTransferByUserIdRequest, UpdateTransferRequest,
    transfer_service_client::TransferServiceClient,
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
            CreateTransferRequest as DomainCreateTransferRequest,
            FindAllTransferRequest as DomainFindAllTransferRequest,
            UpdateTransferRequest as DomainUpdateTransferRequest,
        },
        response::{ApiResponse, ApiResponsePagination, ErrorResponse, transfer::TransferResponse},
    },
    utils::{MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};
use tonic::{Request, transport::Channel};
use tracing::{error, info};

use shared::abstract_trait::TransferServiceTrait;

#[derive(Debug)]
pub struct TransferService {
    client: Arc<Mutex<TransferServiceClient<Channel>>>,
    metrics: Arc<Mutex<Metrics>>,
}

impl TransferService {
    pub async fn new(
        client: Arc<Mutex<TransferServiceClient<Channel>>>,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
    ) -> Self {
        registry.register(
            "transfer_handler_request_counter",
            "total number of requests to the TransferService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "transfer_handler_request_duration",
            "Histogram of request durations for the TransferService",
            metrics.lock().await.request_duration.clone(),
        );

        Self { client, metrics }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("transfer-service-client")
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
impl TransferServiceTrait for TransferService {
    async fn get_transfers(
        &self,
        req: &DomainFindAllTransferRequest,
    ) -> Result<ApiResponsePagination<Vec<TransferResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetAllTransfers",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("operation", "get_all"),
                KeyValue::new("page", req.page.to_string()),
                KeyValue::new("page_size", req.page_size.to_string()),
                KeyValue::new("search", req.search.clone()),
            ],
        );

        let mut request = Request::new(FindAllTransferRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_all_transfer(request).await {
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

    async fn get_transfer(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<TransferResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTransfer",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("operation", "get_transfer"),
                KeyValue::new("id", id.to_string()),
            ],
        );

        let mut request = Request::new(FindTransferByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.find_transfer_by_id(request).await {
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
                        "Failed to retrieve transfer (id: {}): {}",
                        id, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_transfer_users(
        &self,
        user_id: i32,
    ) -> Result<ApiResponse<Option<Vec<TransferResponse>>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTransferUsers",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("operation", "get_transfer_users"),
                KeyValue::new("user_id", user_id.to_string()),
            ],
        );

        let mut request = Request::new(FindTransferByUserIdRequest { user_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self
            .client
            .lock()
            .await
            .find_transfer_by_users_id(request)
            .await
        {
            Ok(resp) => {
                let inner = resp.into_inner();
                let response = ApiResponse {
                    status: inner.status,
                    message: inner.message,
                    data: Some(inner.data.into_iter().map(Into::into).collect()),
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
                        "Failed to retrieve transfers (user_id: {user_id}): {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn get_transfer_user(
        &self,
        user_id: i32,
    ) -> Result<ApiResponse<Option<TransferResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTransferUser",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("operation", "get_transfer_user"),
                KeyValue::new("user_id", user_id.to_string()),
            ],
        );

        let mut request = Request::new(FindTransferByUserIdRequest { user_id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self
            .client
            .lock()
            .await
            .find_transfer_by_user_id(request)
            .await
        {
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
                        "Failed to retrieve transfer (user_id: {user_id}): {}",
                        error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn create_transfer(
        &self,
        input: &DomainCreateTransferRequest,
    ) -> Result<ApiResponse<TransferResponse>, ErrorResponse> {
        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "CreateTransfer",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("operation", "create"),
                KeyValue::new("transfer.from", input.transfer_from as i64),
                KeyValue::new("transfer.to", input.transfer_to as i64),
                KeyValue::new("transfer.amount", input.transfer_amount as i64),
            ],
        );

        let mut request = Request::new(CreateTransferRequest {
            transfer_from: input.transfer_from,
            transfer_to: input.transfer_to,
            transfer_amount: input.transfer_amount,
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.create_transfer(request).await {
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
                    &format!(
                        "Transfer from {} to {} of amount {} created successfully",
                        input.transfer_from, input.transfer_to, input.transfer_amount
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

                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!(
                        "Failed to create transfer from {} to {}: {}",
                        input.transfer_from, input.transfer_to, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }

    async fn update_transfer(
        &self,
        input: &DomainUpdateTransferRequest,
    ) -> Result<ApiResponse<TransferResponse>, ErrorResponse> {
        let method = Method::Put;
        let transfer_id = input.transfer_id;
        let transfer_from = input.transfer_from;
        let transfer_to = input.transfer_to;
        let transfer_amount = input.transfer_amount;

        let tracing_ctx = self.start_tracing(
            "UpdateTransfer",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("operation", "update"),
                KeyValue::new("transfer.id", transfer_id as i64),
                KeyValue::new("transfer.from", transfer_from as i64),
                KeyValue::new("transfer.to", transfer_to as i64),
                KeyValue::new("transfer.amount", transfer_amount as i64),
            ],
        );

        let update_request = UpdateTransferRequest {
            transfer_id,
            transfer_from,
            transfer_to,
            transfer_amount,
        };

        let mut request = Request::new(update_request);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.update_transfer(request).await {
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
                &format!(
                    "Transfer updated successfully (ID: {transfer_id}, from: {transfer_from}, to: {transfer_to})"
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

                self.complete_tracing_error(
                &tracing_ctx,
                method,
                &format!(
                    "Failed to update transfer (ID: {transfer_id}, from: {transfer_from}, to: {transfer_to}): {}",
                    error_response.message
                ),
            )
            .await;

                Err(error_response)
            }
        }
    }

    async fn delete_transfer(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;
        let tracing_ctx = self.start_tracing(
            "DeleteTransfer",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("operation", "delete"),
                KeyValue::new("transfer.id", id as i64),
            ],
        );

        let mut request = Request::new(FindTransferByIdRequest { id });
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.client.lock().await.delete_transfer(request).await {
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
                    &format!("Transfer {id} deleted successfully"),
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
                        "Failed to delete transfer {}: {}",
                        id, error_response.message
                    ),
                )
                .await;

                Err(error_response)
            }
        }
    }
}
