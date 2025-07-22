use crate::{
    abstract_trait::{
        DynSaldoRepository, DynUserRepository, DynWithdrawRepository, WithdrawServiceTrait,
    },
    cache::CacheStore,
    domain::{
        request::{
            CreateWithdrawRequest, FindAllWithdrawRequest, UpdateSaldoWithdraw,
            UpdateWithdrawRequest,
        },
        response::{
            ApiResponse, ApiResponsePagination, ErrorResponse, pagination::Pagination,
            withdraw::WithdrawResponse,
        },
    },
    utils::{AppError, MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
use async_trait::async_trait;
use chrono::Utc;
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

#[derive(Clone)]
pub struct WithdrawService {
    withdraw_repository: DynWithdrawRepository,
    saldo_repository: DynSaldoRepository,
    user_repository: DynUserRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl std::fmt::Debug for WithdrawService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WithdrawService")
            .field("withdraw_repository", &"DynWithdrawRepository")
            .field("saldo_repository", &"DynSaldoRepository")
            .field("user_repository", &"DynUserRepository")
            .finish()
    }
}

impl WithdrawService {
    pub async fn new(
        withdraw_repository: DynWithdrawRepository,
        saldo_repository: DynSaldoRepository,
        user_repository: DynUserRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.register(
            "withdraw_service_request_counter",
            "Total number of requests to the WithdrawService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "withdraw_service_request_duration",
            "Histogram of requests durations for the WithdrawService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            withdraw_repository,
            saldo_repository,
            user_repository,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("withdraw-service")
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
        req: &FindAllWithdrawRequest,
    ) -> Result<ApiResponsePagination<Vec<WithdrawResponse>>, ErrorResponse> {
        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let (withdraws, total_items) = self
            .withdraw_repository
            .find_all(page, page_size, search)
            .await?;

        info!("Found {} withdraws", withdraws.len());

        let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;

        let withdraw_responses: Vec<WithdrawResponse> =
            withdraws.into_iter().map(WithdrawResponse::from).collect();

        Ok(ApiResponsePagination {
            status: "success".to_string(),
            message: "Withdraws retrieved successfully".to_string(),
            data: withdraw_responses,
            pagination: Pagination {
                page,
                page_size,
                total_items,
                total_pages,
            },
        })
    }

    async fn get_withdraw(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<WithdrawResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetWithdraw",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.withdraw_repository.find_by_id(id).await {
            Ok(Some(withdraw)) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Withdraw retrieved successfully".to_string(),
                    data: Some(WithdrawResponse::from(withdraw)),
                };

                info!("Successfully retrieved withdraw with ID: {id}");
                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Withdraw retrieved successfully",
                )
                .await;

                Ok(response)
            }
            Ok(None) => {
                let msg = format!("Withdraw with id {id} not found");
                error!("{}", msg);

                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
            Err(err) => {
                let msg = format!("Failed to retrieve withdraw: {err}");
                error!("{}", msg);

                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn get_withdraw_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<WithdrawResponse>>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetWithdrawUsers",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let user_result = self.user_repository.find_by_id(id).await;
        let _user = match user_result {
            Ok(user) => user,
            Err(_) => {
                let msg = format!("User with id {id} not found");
                error!("{}", msg);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let withdraw_result = self.withdraw_repository.find_by_users(id).await;

        let withdraws = match withdraw_result {
            Ok(w) => w,
            Err(err) => {
                let msg = format!("Failed to retrieve withdraws for user {id}: {err}");
                error!("{}", msg);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        let withdraw_response: Option<Vec<WithdrawResponse>> = if withdraws.is_empty() {
            None
        } else {
            Some(withdraws.into_iter().map(WithdrawResponse::from).collect())
        };

        let response = if let Some(ref data) = withdraw_response {
            let response = ApiResponse {
                status: "success".to_string(),
                data: Some(data.clone()),
                message: "Withdraw retrieved successfully".to_string(),
            };

            self.complete_tracing_success(&tracing_ctx, method, "Withdraw retrieved from database")
                .await;

            response
        } else {
            let response = ApiResponse {
                status: "success".to_string(),
                data: None,
                message: format!("No withdraw found for user with id {id}"),
            };

            self.complete_tracing_success(&tracing_ctx, method, "No withdraw found")
                .await;

            response
        };

        Ok(response)
    }

    async fn get_withdraw_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<WithdrawResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetWithdrawUser",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("withdraw_user:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<Option<WithdrawResponse>>>(&cache_key)
        {
            info!("Found withdraw in cache for user_id: {id}");

            self.complete_tracing_success(&tracing_ctx, method, "Withdraw retrieved from cache")
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

        let withdraw_result = self.withdraw_repository.find_by_user(id).await;
        let withdraw_opt = match withdraw_result {
            Ok(w) => w.map(WithdrawResponse::from),
            Err(err) => {
                let msg = format!("Failed to retrieve withdraw for user_id {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        match withdraw_opt {
            Some(withdraw) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Withdraw retrieved successfully".to_string(),
                    data: Some(withdraw.clone()),
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Withdraw retrieved from database",
                )
                .await;

                Ok(response)
            }
            None => {
                let msg = format!("No withdraw found for user_id: {id}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
        }
    }

    async fn create_withdraw(
        &self,
        input: &CreateWithdrawRequest,
    ) -> Result<ApiResponse<WithdrawResponse>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "CreateWithdraw",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("user_id", input.user_id.to_string()),
                KeyValue::new("withdraw_amount", input.withdraw_amount.to_string()),
            ],
        );

        let mut request = Request::new(input.user_id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        info!("Creating withdraw for user_id: {}", input.user_id);

        let saldo_opt = match self.saldo_repository.find_by_user_id(input.user_id).await {
            Ok(s) => s,
            Err(_) => {
                let msg = format!("Saldo with user_id {} not found", input.user_id);
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let saldo_ref = match saldo_opt {
            Some(ref s) => s,
            None => {
                let msg = format!("Saldo not found for user_id: {}", input.user_id);
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        info!(
            "Saldo found for user_id: {}. Current balance: {}",
            input.user_id, saldo_ref.total_balance
        );

        if saldo_ref.total_balance < input.withdraw_amount {
            let msg = format!(
                "Insufficient balance for user_id: {}. Attempted withdrawal: {}",
                input.user_id, input.withdraw_amount
            );
            error!("{msg}");
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;
            return Err(ErrorResponse::from(AppError::Custom(
                "Insufficient balance".to_string(),
            )));
        }

        info!("User has sufficient balance for withdrawal");

        let new_total_balance = saldo_ref.total_balance - input.withdraw_amount;

        let _update_saldo_balance = match self
            .saldo_repository
            .update_saldo_withdraw(&UpdateSaldoWithdraw {
                user_id: input.user_id,
                withdraw_amount: Some(input.withdraw_amount),
                withdraw_time: Some(Utc::now()),
                total_balance: new_total_balance,
            })
            .await
        {
            Ok(s) => s,
            Err(err) => {
                let msg = format!("Failed to update saldo: {err}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        info!(
            "Saldo balance updated for user_id: {}. New balance: {new_total_balance}",
            input.user_id
        );

        let withdraw_create_result = match self.withdraw_repository.create(input).await {
            Ok(w) => w,
            Err(err) => {
                let msg = format!("Failed to create withdraw: {err}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        info!(
            "Withdraw created successfully for user_id: {}",
            input.user_id
        );

        self.complete_tracing_success(&tracing_ctx, method, "Withdraw created successfully")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Withdraw created successfully".to_string(),
            data: withdraw_create_result.into(),
        })
    }

    async fn update_withdraw(
        &self,
        input: &UpdateWithdrawRequest,
    ) -> Result<ApiResponse<WithdrawResponse>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "UpdateWithdraw",
            vec![
                KeyValue::new("component", "withdraw"),
                KeyValue::new("user_id", input.user_id.to_string()),
                KeyValue::new("withdraw_id", input.withdraw_id.to_string()),
                KeyValue::new("withdraw_amount", input.withdraw_amount.to_string()),
            ],
        );

        let mut request = Request::new(input.user_id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let _withdraw = match self.withdraw_repository.find_by_id(input.withdraw_id).await {
            Ok(w) => w,
            Err(_) => {
                let msg = format!("Withdraw with id {} not found", input.withdraw_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let saldo_opt = match self.saldo_repository.find_by_user_id(input.user_id).await {
            Ok(s) => s,
            Err(_) => {
                let msg = format!("Saldo with user_id {} not found", input.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let saldo_ref = match saldo_opt {
            Some(ref s) => s,
            None => {
                let msg = format!("Saldo not found for user_id: {}", input.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let new_total_balance = saldo_ref.total_balance - input.withdraw_amount;

        let updated_withdraw = self.withdraw_repository.update(input).await;

        if let Err(err) = updated_withdraw {
            let _rollback_saldo = self
                .saldo_repository
                .update_saldo_withdraw(&UpdateSaldoWithdraw {
                    user_id: input.user_id,
                    withdraw_amount: None,
                    withdraw_time: None,
                    total_balance: saldo_ref.total_balance,
                })
                .await?;

            let msg = format!("Withdraw update failed: {err}");
            error!("Rollback: {msg}");

            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;
            return Err(err.into());
        }

        let _update_saldo = match self
            .saldo_repository
            .update_saldo_withdraw(&UpdateSaldoWithdraw {
                user_id: input.user_id,
                withdraw_amount: Some(input.withdraw_amount),
                withdraw_time: Some(Utc::now()),
                total_balance: new_total_balance,
            })
            .await
        {
            Ok(v) => v,
            Err(err) => {
                let msg = format!("Failed to update saldo: {err}");
                error!("{msg}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        self.complete_tracing_success(&tracing_ctx, method, "Withdraw updated successfully")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Withdraw updated successfully".to_string(),
            data: updated_withdraw.unwrap().into(),
        })
    }

    async fn delete_withdraw(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "DeleteWithdraw",
            vec![
                KeyValue::new("component", "withdraw"),
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

        let existing = match self.withdraw_repository.find_by_user(user.user_id).await {
            Ok(Some(withdraw)) => withdraw,
            Ok(None) => {
                let msg = format!("Withdraw with user_id {} not found", user.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
            Err(err) => {
                let msg = format!(
                    "Failed to find withdraw for user_id {}: {err}",
                    user.user_id
                );
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        if let Err(err) = self.withdraw_repository.delete(existing.withdraw_id).await {
            let msg = format!(
                "Failed to delete withdraw id {}: {}",
                existing.withdraw_id, err
            );
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;
            return Err(ErrorResponse::from(err));
        }

        let cache_key = format!("withdraw_user:id={}", user.user_id);
        self.cache_store.delete_from_cache(&cache_key);

        info!(
            "Withdraw deleted successfully for user_id: {}",
            user.user_id
        );

        self.complete_tracing_success(&tracing_ctx, method, "Withdraw deleted successfully")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Withdraw deleted successfully".to_string(),
            data: (),
        })
    }
}
