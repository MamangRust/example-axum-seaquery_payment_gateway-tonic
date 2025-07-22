use async_trait::async_trait;
use opentelemetry::{
    Context, KeyValue,
    global::{self, BoxedTracer},
    trace::{Span, SpanKind, TraceContextExt, Tracer},
};
use prometheus_client::registry::Registry;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};
use tonic::Request;
use tracing::{error, info};

use crate::{
    abstract_trait::{
        DynSaldoRepository, DynTransferRepository, DynUserRepository, TransferServiceTrait,
    },
    cache::CacheStore,
    domain::{
        request::{
            CreateTransferRequest, FindAllTransferRequest, UpdateSaldoBalance,
            UpdateTransferRequest,
        },
        response::{
            ApiResponse, ApiResponsePagination, ErrorResponse, pagination::Pagination,
            transfer::TransferResponse,
        },
    },
    utils::{AppError, MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};

#[derive(Clone)]
pub struct TransferService {
    transfer_repository: DynTransferRepository,
    saldo_repository: DynSaldoRepository,
    user_repository: DynUserRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl std::fmt::Debug for TransferService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransferService")
            .field("transfer_repository", &"DynTransferRepository")
            .field("saldo_repository", &"DynSaldoRepository")
            .field("user_repository", &"DynUserRepository")
            .finish()
    }
}

impl TransferService {
    pub async fn new(
        transfer_repository: DynTransferRepository,
        saldo_repository: DynSaldoRepository,
        user_repository: DynUserRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.register(
            "transfer_service_request_counter",
            "Total number of requests to the TransferService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "transfer_service_request_duration",
            "Histogram of request durations for the TransferService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            transfer_repository,
            saldo_repository,
            user_repository,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("transfer-service")
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
        req: &FindAllTransferRequest,
    ) -> Result<ApiResponsePagination<Vec<TransferResponse>>, ErrorResponse> {
        let method = Method::Get;

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let tracing_ctx = self.start_tracing(
            "GetTransfers",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(FindAllTransferRequest {
            page,
            page_size,
            search: search.clone().unwrap_or_default(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "transfers:page={page}:size={page_size}:search={}",
            search.clone().unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<TransferResponse>>>(&cache_key)
        {
            info!("Found transfers in cache");

            self.complete_tracing_success(&tracing_ctx, method, "transfers retrieved from cache")
                .await;

            return Ok(cached);
        }

        match self
            .transfer_repository
            .find_all(page, page_size, search)
            .await
        {
            Ok((transfers, total_items)) => {
                let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;
                let category_responses = transfers
                    .into_iter()
                    .map(TransferResponse::from)
                    .collect::<Vec<_>>();

                let response = ApiResponsePagination {
                    status: "success".to_string(),
                    message: "transfers retrieved successfully".to_string(),
                    data: category_responses.clone(),
                    pagination: Pagination {
                        page,
                        page_size,
                        total_items,
                        total_pages,
                    },
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "transfers retrieved from database",
                )
                .await;

                Ok(response)
            }

            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to retrieve transfers: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
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
                KeyValue::new("id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("transfer:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<Option<TransferResponse>>>(&cache_key)
        {
            info!("Found transfer in cache");

            self.complete_tracing_success(&tracing_ctx, method, "Transfer retrieved from cache")
                .await;

            return Ok(cached);
        }

        match self.transfer_repository.find_by_id(id).await {
            Ok(Some(transfer)) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Transfer retrieved successfully".to_string(),
                    data: Some(TransferResponse::from(transfer)),
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Transfer retrieved from database",
                )
                .await;

                Ok(response)
            }
            Ok(None) => {
                let msg = format!("Transfer with id {id} not found");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
            Err(err) => {
                let msg = format!("Failed to retrieve transfer: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn get_transfer_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<TransferResponse>>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTransferUsers",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("transfer_users:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<Option<Vec<TransferResponse>>>>(&cache_key)
        {
            info!("Found user transfer in cache");

            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "User transfer retrieved from cache",
            )
            .await;

            return Ok(cached);
        }

        let user_result = self.user_repository.find_by_id(id).await;

        let _user = match user_result {
            Ok(user) => user,
            Err(_) => {
                let msg = format!("User with id {id} not found");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let transfer_result = self.transfer_repository.find_by_users(id).await;

        let transfer = match transfer_result {
            Ok(t) => t,
            Err(err) => {
                let msg = format!("Failed to retrieve transfer for user {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        let transfer_responses = if transfer.is_empty() {
            None
        } else {
            Some(
                transfer
                    .into_iter()
                    .map(TransferResponse::from)
                    .collect::<Vec<_>>(),
            )
        };

        let response = if let Some(ref data) = transfer_responses {
            let response = ApiResponse {
                status: "success".to_string(),
                data: Some(data.clone()),
                message: "Success".to_string(),
            };

            self.cache_store
                .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "User transfer retrieved from database",
            )
            .await;

            response
        } else {
            let response = ApiResponse {
                status: "success".to_string(),
                data: None,
                message: format!("No transfer found for user with id {id}"),
            };

            self.complete_tracing_success(&tracing_ctx, method, "No transfer found")
                .await;

            response
        };

        Ok(response)
    }

    async fn get_transfer_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<TransferResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTransferUser",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("transfer_user:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<Option<TransferResponse>>>(&cache_key)
        {
            info!("Found transfer in cache for user_id: {id}");

            self.complete_tracing_success(&tracing_ctx, method, "Transfer retrieved from cache")
                .await;

            return Ok(cached);
        }

        let user_result = self.user_repository.find_by_id(id).await;
        let _user = match user_result {
            Ok(user) => user,
            Err(_) => {
                let msg = format!("User with id {id} not found");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let transfer_result = self.transfer_repository.find_by_user(id).await;
        let transfer_opt = match transfer_result {
            Ok(t) => t.map(TransferResponse::from),
            Err(err) => {
                let msg = format!("Failed to retrieve transfer for user_id {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        match transfer_opt {
            Some(transfer) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Transfer retrieved successfully".to_string(),
                    data: Some(transfer.clone()),
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Transfer retrieved from database",
                )
                .await;

                Ok(response)
            }
            None => {
                let msg = format!("No transfer found for user_id: {id}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
        }
    }

    async fn create_transfer(
        &self,
        input: &CreateTransferRequest,
    ) -> Result<ApiResponse<TransferResponse>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "CreateTransfer",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("from_user_id", input.transfer_from.to_string()),
                KeyValue::new("to_user_id", input.transfer_to.to_string()),
                KeyValue::new("amount", input.transfer_amount.to_string()),
            ],
        );

        let mut request = Request::new(input.transfer_from);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let _sender = match self.user_repository.find_by_id(input.transfer_from).await {
            Ok(user) => user,
            Err(_) => {
                let msg = format!("User with id {} not found", input.transfer_from);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let _receiver = match self.user_repository.find_by_id(input.transfer_to).await {
            Ok(user) => user,
            Err(_) => {
                let msg = format!("User with id {} not found", input.transfer_to);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let transfer = self.transfer_repository.create(input).await?;

        let sender_saldo = match self
            .saldo_repository
            .find_by_user_id(input.transfer_from)
            .await
        {
            Ok(saldo) => saldo,
            Err(_) => {
                let msg = format!("Saldo with User id {} not found", input.transfer_from);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let sender_balance = sender_saldo.unwrap().total_balance - input.transfer_amount;

        let request_sender_balance = UpdateSaldoBalance {
            user_id: input.transfer_from,
            total_balance: sender_balance,
        };

        if let Err(db_err) = self
            .saldo_repository
            .update_balance(&request_sender_balance)
            .await
        {
            error!("Failed to update saldo balance for sender: {db_err}");
            let msg = "Failed to update saldo balance for sender".to_string();
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;

            self.transfer_repository
                .delete(transfer.transfer_id)
                .await?;
            return Err(db_err.into());
        }

        let receiver_saldo = match self
            .saldo_repository
            .find_by_user_id(input.transfer_to)
            .await
        {
            Ok(saldo) => saldo,
            Err(_) => {
                let msg = format!("Saldo with User id {} not found", input.transfer_to);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let receiver_balance = receiver_saldo.unwrap().total_balance + input.transfer_amount;

        let request_receiver_balance = UpdateSaldoBalance {
            user_id: input.transfer_to,
            total_balance: receiver_balance,
        };

        if let Err(db_err) = self
            .saldo_repository
            .update_balance(&request_receiver_balance)
            .await
        {
            error!("Failed to update saldo balance for receiver: {db_err}");
            let msg = "Failed to update saldo balance for receiver".to_string();
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;

            let _ = self.transfer_repository.delete(transfer.transfer_id).await;
            return Err(db_err.into());
        }

        self.complete_tracing_success(&tracing_ctx, method, "Transfer created successfully")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Transfer created successfully".to_string(),
            data: TransferResponse::from(transfer),
        })
    }

    async fn update_transfer(
        &self,
        input: &UpdateTransferRequest,
    ) -> Result<ApiResponse<TransferResponse>, ErrorResponse> {
        let method = Method::Put;

        let tracing_ctx = self.start_tracing(
            "UpdateTransfer",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("transfer_id", input.transfer_id.to_string()),
                KeyValue::new("new_amount", input.transfer_amount.to_string()),
            ],
        );

        let mut request = Request::new(input.transfer_id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let transfer = match self.transfer_repository.find_by_id(input.transfer_id).await {
            Ok(Some(t)) => t,
            _ => {
                let msg = format!("Transfer with id {} not found", input.transfer_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let amount_difference = input.transfer_amount as i64 - transfer.transfer_amount as i64;

        let sender_saldo = match self
            .saldo_repository
            .find_by_user_id(transfer.transfer_from)
            .await
        {
            Ok(Some(saldo)) => saldo,
            _ => {
                let msg = format!("Saldo with User id {} not found", transfer.transfer_from);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let new_sender_balance = sender_saldo.total_balance - amount_difference as i32;

        if new_sender_balance < 0 {
            let msg = "Insufficient balance for sender".to_string();
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;
            return Err(ErrorResponse::from(AppError::Custom(msg)));
        }

        let update_sender_balance = UpdateSaldoBalance {
            user_id: transfer.transfer_from,
            total_balance: new_sender_balance,
        };

        if let Err(db_err) = self
            .saldo_repository
            .update_balance(&update_sender_balance)
            .await
        {
            let msg = format!("Failed to update sender's saldo: {db_err}");
            error!("{msg}");
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;
            return Err(db_err.into());
        }

        let receiver_saldo = match self
            .saldo_repository
            .find_by_user_id(transfer.transfer_to)
            .await
        {
            Ok(Some(saldo)) => saldo,
            _ => {
                let msg = format!("Saldo with User id {} not found", transfer.transfer_to);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let new_receiver_balance = receiver_saldo.total_balance + amount_difference as i32;

        let update_receiver_balance = UpdateSaldoBalance {
            user_id: transfer.transfer_to,
            total_balance: new_receiver_balance,
        };

        if let Err(db_err) = self
            .saldo_repository
            .update_balance(&update_receiver_balance)
            .await
        {
            let msg = format!("Failed to update receiver's saldo: {db_err}");
            error!("{msg}");
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;

            let rollback_sender_balance = UpdateSaldoBalance {
                user_id: transfer.transfer_from,
                total_balance: sender_saldo.total_balance,
            };

            if let Err(rollback_err) = self
                .saldo_repository
                .update_balance(&rollback_sender_balance)
                .await
            {
                error!("Failed to rollback sender's saldo update: {rollback_err}");
            }

            return Err(db_err.into());
        }

        let updated_transfer = self.transfer_repository.update(input).await?;

        let msg = format!("Transfer updated successfully: id {}", input.transfer_id);
        self.complete_tracing_success(&tracing_ctx, method, &msg)
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Transfer updated successfully".to_string(),
            data: TransferResponse::from(updated_transfer),
        })
    }

    async fn delete_transfer(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "DeleteTransfer",
            vec![
                KeyValue::new("component", "transfer"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let user = match self.user_repository.find_by_id(id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                let msg = format!("User with id {id} not found");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
            Err(err) => {
                let msg = format!("Failed to fetch user {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        let existing_transfer = match self.transfer_repository.find_by_user(user.user_id).await {
            Ok(Some(transfer)) => transfer,
            Ok(None) => {
                let msg = format!("Transfer with user_id {} not found", user.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
            Err(err) => {
                let msg = format!(
                    "Failed to find transfer for user_id {}: {err}",
                    user.user_id
                );
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        if let Err(err) = self
            .transfer_repository
            .delete(existing_transfer.transfer_id)
            .await
        {
            let msg = format!(
                "Failed to delete transfer id {}: {}",
                existing_transfer.transfer_id, err
            );
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;
            return Err(ErrorResponse::from(err));
        }

        info!(
            "Transfer deleted successfully for user_id: {}",
            user.user_id
        );

        self.complete_tracing_success(&tracing_ctx, method, "Transfer deleted successfully")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Transfer deleted successfully".to_string(),
            data: (),
        })
    }
}
