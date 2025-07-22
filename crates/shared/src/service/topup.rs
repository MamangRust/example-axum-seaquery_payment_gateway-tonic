use crate::{
    abstract_trait::{
        DynSaldoRepository, DynTopupRepository, DynUserRepository, TopupServiceTrait,
    },
    cache::CacheStore,
    domain::{
        request::{
            CreateSaldoRequest, CreateTopupRequest, FindAllTopupRequest, UpdateSaldoBalance,
            UpdateTopupAmount, UpdateTopupRequest,
        },
        response::{
            ApiResponse, ApiResponsePagination, ErrorResponse, pagination::Pagination,
            topup::TopupResponse,
        },
    },
    utils::{AppError, MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};
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

pub struct TopupService {
    topup_repository: DynTopupRepository,
    saldo_repository: DynSaldoRepository,
    user_repository: DynUserRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl std::fmt::Debug for TopupService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TopupService")
            .field("topup_repository", &"DynTopupRepository")
            .field("saldo_repository", &"DynSaldoRepository")
            .field("user_repository", &"DynUserRepository")
            .finish()
    }
}

impl TopupService {
    pub async fn new(
        topup_repository: DynTopupRepository,
        saldo_repository: DynSaldoRepository,
        user_repository: DynUserRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.register(
            "category_service_request_counter",
            "Total number of requests to the CategoryService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "category_service_request_duration",
            "Histogram of request durations for the CategoryService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            topup_repository,
            saldo_repository,
            user_repository,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("topup-service")
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
        req: &FindAllTopupRequest,
    ) -> Result<ApiResponsePagination<Vec<TopupResponse>>, ErrorResponse> {
        let method = Method::Get;

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let tracing_ctx = self.start_tracing(
            "Gettopups",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(FindAllTopupRequest {
            page,
            page_size,
            search: search.clone().unwrap_or_default(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "topups:page={page}:size={page_size}:search={}",
            search.clone().unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<TopupResponse>>>(&cache_key)
        {
            info!("Found topups in cache");

            self.complete_tracing_success(&tracing_ctx, method, "topups retrieved from cache")
                .await;

            return Ok(cached);
        }

        match self
            .topup_repository
            .find_all(page, page_size, search)
            .await
        {
            Ok((topups, total_items)) => {
                let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;
                let topup_responses = topups
                    .into_iter()
                    .map(TopupResponse::from)
                    .collect::<Vec<_>>();

                let response = ApiResponsePagination {
                    status: "success".to_string(),
                    message: "topups retrieved successfully".to_string(),
                    data: topup_responses.clone(),
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
                    "topups retrieved from database",
                )
                .await;

                Ok(response)
            }

            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to retrieve topups: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
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
                KeyValue::new("topup_id", id.to_string()),
            ],
        );

        let cache_key = format!("topup:id={id}");

        if let Some(cached) = self.cache_store.get_from_cache::<TopupResponse>(&cache_key) {
            info!("Topup with id {id} found in cache");

            self.complete_tracing_success(&tracing_ctx, method, "Topup retrieved from cache")
                .await;

            return Ok(ApiResponse {
                status: "success".to_string(),
                message: "Topup retrieved successfully (from cache)".to_string(),
                data: Some(cached),
            });
        }

        let topup = self.topup_repository.find_by_id(id).await;

        match topup {
            Ok(Some(topup)) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Topup retrieved successfully".to_string(),
                    data: Some(TopupResponse::from(topup)),
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                info!("Successfully retrieved topup with id {id}");

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Topup retrieved from database",
                )
                .await;

                Ok(response)
            }
            Ok(None) => {
                let msg = format!("Topup with id {id} not found");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                error!("{msg}");

                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
            Err(err) => {
                let msg = format!("Error fetching topup with id {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(err.into())
            }
        }
    }

    async fn get_topup_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<TopupResponse>>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTopupUsers",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("topup_users:id={id}");

        // Coba ambil dari cache
        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<Option<Vec<TopupResponse>>>>(&cache_key)
        {
            info!("Found user topups in cache");

            self.complete_tracing_success(&tracing_ctx, method, "User topups retrieved from cache")
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

        let topup_result = self.topup_repository.find_by_users(id).await;

        let topups = match topup_result {
            Ok(t) => t,
            Err(err) => {
                let msg = format!("Failed to retrieve topups for user {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        let topup_responses = if topups.is_empty() {
            None
        } else {
            Some(
                topups
                    .into_iter()
                    .map(TopupResponse::from)
                    .collect::<Vec<_>>(),
            )
        };

        let response = if let Some(ref data) = topup_responses {
            let response = ApiResponse {
                status: "success".to_string(),
                message: "Success".to_string(),
                data: Some(data.clone()),
            };

            self.cache_store
                .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "User topups retrieved from database",
            )
            .await;

            response
        } else {
            let response = ApiResponse {
                status: "success".to_string(),
                message: format!("No topup found for user with id {id}"),
                data: None,
            };

            self.complete_tracing_success(&tracing_ctx, method, "No topup found")
                .await;

            response
        };

        Ok(response)
    }

    async fn get_topup_user(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<TopupResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetTopupUser",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("topup_user:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<Option<TopupResponse>>>(&cache_key)
        {
            info!("Found topup in cache for user_id: {id}");

            self.complete_tracing_success(&tracing_ctx, method, "Topup retrieved from cache")
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

        let topup_result = self.topup_repository.find_by_user(id).await;
        let topup_opt = match topup_result {
            Ok(topup) => topup.map(TopupResponse::from),
            Err(err) => {
                let msg = format!("Failed to retrieve topup for user_id {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        match topup_opt {
            Some(topup) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Topup retrieved successfully".to_string(),
                    data: Some(topup.clone()),
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Topup retrieved from database",
                )
                .await;

                Ok(response)
            }
            None => {
                let msg = format!("No topup found for user_id: {id}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
        }
    }

    async fn create_topup(
        &self,
        input: &CreateTopupRequest,
    ) -> Result<ApiResponse<TopupResponse>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "CreateTopup",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("user_id", input.user_id.to_string()),
                KeyValue::new("topup_amount", input.topup_amount.to_string()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let _user = match self.user_repository.find_by_id(input.user_id).await {
            Ok(user) => user,
            Err(_) => {
                let msg = format!("User with id {} not found", input.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let topup = self.topup_repository.create(input).await?;

        match self.saldo_repository.find_by_user_id(input.user_id).await {
            Ok(Some(current_saldo)) => {
                let new_balance = current_saldo.total_balance + topup.topup_amount;
                let request = UpdateSaldoBalance {
                    user_id: input.user_id,
                    total_balance: new_balance,
                };

                if let Err(db_err) = self.saldo_repository.update_balance(&request).await {
                    let msg = format!(
                        "Failed to update saldo balance for user {}: {}",
                        input.user_id, db_err
                    );
                    error!("{msg}");

                    if let Err(rb_err) = self.topup_repository.delete(topup.topup_id).await {
                        error!(
                            "Failed to rollback topup creation for user {}: {}",
                            input.user_id, rb_err
                        );
                    }

                    self.complete_tracing_error(&tracing_ctx, method, &msg)
                        .await;

                    return Err(db_err.into());
                }
            }
            Ok(None) => {
                let create_saldo_request = CreateSaldoRequest {
                    user_id: input.user_id,
                    total_balance: topup.topup_amount,
                };

                if let Err(db_err) = self.saldo_repository.create(&create_saldo_request).await {
                    let msg = format!(
                        "Failed to create initial saldo for user {}: {db_err}",
                        input.user_id,
                    );

                    error!("{msg}");

                    if let Err(rb_err) = self.topup_repository.delete(topup.topup_id).await {
                        error!(
                            "Failed to rollback topup creation for user {}: {rb_err}",
                            input.user_id,
                        );
                    }

                    self.complete_tracing_error(&tracing_ctx, method, &msg)
                        .await;

                    return Err(db_err.into());
                }
            }
            Err(err) => {
                let msg = format!(
                    "Failed to retrieve saldo for user {}: {}",
                    input.user_id, err
                );
                error!("{msg}");

                if let Err(rb_err) = self.topup_repository.delete(topup.topup_id).await {
                    error!(
                        "Failed to rollback topup creation for user {}: {}",
                        input.user_id, rb_err
                    );
                }

                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                return Err(ErrorResponse::from(AppError::Custom(msg.clone())));
            }
        }

        let message = format!(
            "Topup successfully created for user {}. Total balance updated.",
            input.user_id
        );
        info!("{message}");

        self.complete_tracing_success(&tracing_ctx, method, &message)
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Topup created successfully".to_string(),
            data: TopupResponse::from(topup),
        })
    }

    async fn update_topup(
        &self,
        input: &UpdateTopupRequest,
    ) -> Result<ApiResponse<TopupResponse>, ErrorResponse> {
        let method = Method::Put;

        let tracing_ctx = self.start_tracing(
            "UpdateTopup",
            vec![
                KeyValue::new("component", "topup"),
                KeyValue::new("user_id", input.user_id.to_string()),
                KeyValue::new("topup_id", input.topup_id.to_string()),
            ],
        );

        let _user = match self.user_repository.find_by_id(input.user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                let msg = format!("User with id {} not found", input.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
            Err(err) => {
                let msg = format!("Failed to fetch user {}: {}", input.user_id, err);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        info!(
            "User with id {} found, proceeding with topup update",
            input.user_id
        );

        let existing_topup = match self.topup_repository.find_by_id(input.topup_id).await {
            Ok(Some(topup)) => topup,
            Ok(None) => {
                let msg = format!("Topup with id {} not found", input.topup_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
            Err(err) => {
                let msg = format!("Failed to fetch topup {}: {}", input.topup_id, err);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        let topup_difference = input.topup_amount - existing_topup.topup_amount;

        info!(
            "Calculating topup difference: new amount {} - old amount {} = difference {topup_difference}",
            input.topup_amount, existing_topup.topup_amount,
        );

        let update_topup = UpdateTopupAmount {
            topup_id: input.topup_id,
            topup_amount: input.topup_amount,
        };

        if let Err(err) = self.topup_repository.update_amount(&update_topup).await {
            let msg = format!(
                "Failed to update topup {} for user {}: {}",
                input.topup_id, input.user_id, err
            );
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;
            return Err(ErrorResponse::from(err));
        }

        match self.saldo_repository.find_by_user_id(input.user_id).await {
            Ok(Some(current_saldo)) => {
                let new_balance = current_saldo.total_balance + topup_difference;

                info!(
                    "Updating saldo: current balance {} + topup difference {topup_difference} = new balance {new_balance}",
                    current_saldo.total_balance,
                );

                let request = UpdateSaldoBalance {
                    user_id: input.user_id,
                    total_balance: new_balance,
                };

                if let Err(db_err) = self.saldo_repository.update_balance(&request).await {
                    let msg = format!(
                        "Failed to update saldo balance for user {}: {}",
                        input.user_id, db_err
                    );
                    self.complete_tracing_error(&tracing_ctx, method, &msg)
                        .await;

                    let rollback = UpdateTopupAmount {
                        topup_id: existing_topup.topup_id,
                        topup_amount: existing_topup.topup_amount,
                    };

                    if let Err(rb_err) = self.topup_repository.update_amount(&rollback).await {
                        error!(
                            "Failed to rollback topup update for user {}: {}",
                            input.user_id, rb_err
                        );
                    }

                    return Err(db_err.into());
                }

                info!(
                    "Saldo updated successfully for user {}. New balance: {}",
                    input.user_id, new_balance
                );
            }
            Ok(None) => {
                let msg = format!("Saldo for user {} not found", input.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
            Err(e) => {
                let msg = format!("Failed to retrieve saldo for user {}: {}", input.user_id, e);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(e.into());
            }
        }

        let updated_topup = self.topup_repository.find_by_id(input.topup_id).await?;

        match updated_topup {
            Some(topup) => {
                self.complete_tracing_success(&tracing_ctx, method, "Topup updated successfully")
                    .await;
                Ok(ApiResponse {
                    status: "success".to_string(),
                    message: "Topup updated successfully".to_string(),
                    data: TopupResponse::from(topup),
                })
            }
            None => {
                let msg = format!("Topup with id {} not found", input.topup_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
        }
    }

    async fn delete_topup(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let user = self.user_repository.find_by_id(id).await.map_err(|_| {
            ErrorResponse::from(AppError::NotFound(format!("User with id {id} not found")))
        })?;

        let existing_topup = self
            .topup_repository
            .find_by_user(user.unwrap().user_id)
            .await?;

        match existing_topup {
            Some(_) => {
                self.topup_repository
                    .delete(existing_topup.unwrap().topup_id)
                    .await?;

                info!("Topup deleted successfully for id: {id}");

                Ok(ApiResponse {
                    status: "success".to_string(),
                    message: "Topup deleted successfully".to_string(),
                    data: (),
                })
            }
            None => {
                error!("Topup with id {id} not found");
                Err(ErrorResponse::from(AppError::NotFound(format!(
                    "Topup with id {id} not found",
                ))))
            }
        }
    }
}
