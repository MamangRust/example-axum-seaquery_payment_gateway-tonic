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
    abstract_trait::{DynSaldoRepository, DynUserRepository, SaldoServiceTrait},
    cache::CacheStore,
    domain::{
        request::{CreateSaldoRequest, FindAllSaldoRequest, UpdateSaldoRequest},
        response::{
            ApiResponse, ApiResponsePagination, ErrorResponse, pagination::Pagination,
            saldo::SaldoResponse,
        },
    },
    utils::{AppError, MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext},
};

#[derive(Clone)]
pub struct SaldoService {
    user_repository: DynUserRepository,
    saldo_repository: DynSaldoRepository,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl std::fmt::Debug for SaldoService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaldoService")
            .field("user_repository", &"DynUserRepository")
            .field("saldo_repository", &"DynSaldoRepository")
            .finish()
    }
}

impl SaldoService {
    pub async fn new(
        user_repository: DynUserRepository,
        saldo_repository: DynSaldoRepository,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.register(
            "saldo_service_request_counter",
            "Total number of requests to the SaldoService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "saldo_service_request_duration",
            "Histogram of requests durations for the SaldoService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            user_repository,
            saldo_repository,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("saldo-service")
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
        req: &FindAllSaldoRequest,
    ) -> Result<ApiResponsePagination<Vec<SaldoResponse>>, ErrorResponse> {
        let method = Method::Get;

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let tracing_ctx = self.start_tracing(
            "Getsaldos",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(FindAllSaldoRequest {
            page,
            page_size,
            search: search.clone().unwrap_or_default(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "saldos:page={page}:size={page_size}:search={}",
            search.clone().unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<SaldoResponse>>>(&cache_key)
        {
            info!("Found saldos in cache");

            self.complete_tracing_success(&tracing_ctx, method, "Saldos retrieved from cache")
                .await;

            return Ok(cached);
        }

        match self
            .saldo_repository
            .find_all(page, page_size, search)
            .await
        {
            Ok((saldos, total_items)) => {
                let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;
                let category_responses = saldos
                    .into_iter()
                    .map(SaldoResponse::from)
                    .collect::<Vec<_>>();

                let response = ApiResponsePagination {
                    status: "success".to_string(),
                    message: "saldos retrieved successfully".to_string(),
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
                    "saldos retrieved from database",
                )
                .await;

                Ok(response)
            }

            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to retrieve saldos: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn get_saldo(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<SaldoResponse>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "Getsaldo",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("saldo:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<Option<SaldoResponse>>>(&cache_key)
        {
            info!("Found saldo in cache");

            self.complete_tracing_success(&tracing_ctx, method, "Saldo retrieved from cache")
                .await;

            return Ok(cached);
        }

        match self.saldo_repository.find_by_id(id).await {
            Ok(Some(saldo)) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Saldo retrieved successfully".to_string(),
                    data: Some(SaldoResponse::from(saldo)),
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Saldo retrieved from database",
                )
                .await;

                Ok(response)
            }
            Ok(None) => {
                let msg = format!("Saldo with id {id} not found");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
            Err(err) => {
                let msg = format!("Failed to retrieve saldo: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn get_saldo_users(
        &self,
        id: i32,
    ) -> Result<ApiResponse<Option<Vec<SaldoResponse>>>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetSaldoUsers",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("saldo_users:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<Option<Vec<SaldoResponse>>>>(&cache_key)
        {
            info!("Found user saldo in cache");

            self.complete_tracing_success(&tracing_ctx, method, "User saldo retrieved from cache")
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

        let saldo_result = self.saldo_repository.find_by_users_id(id).await;

        let saldo = match saldo_result {
            Ok(s) => s,
            Err(err) => {
                let msg = format!("Failed to retrieve saldo for user {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        let saldo_responses = if saldo.is_empty() {
            None
        } else {
            Some(
                saldo
                    .into_iter()
                    .map(SaldoResponse::from)
                    .collect::<Vec<_>>(),
            )
        };

        let response = if let Some(ref data) = saldo_responses {
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
                "User saldo retrieved from database",
            )
            .await;

            response
        } else {
            let response = ApiResponse {
                status: "success".to_string(),
                data: None,
                message: format!("No saldo found for user with id {id}"),
            };

            self.complete_tracing_success(&tracing_ctx, method, "No saldo found")
                .await;
            response
        };

        Ok(response)
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
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        let mut request = Request::new(id);
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("saldo_user:id={id}");

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponse<Option<SaldoResponse>>>(&cache_key)
        {
            info!("Found saldo in cache for user_id: {id}");

            self.complete_tracing_success(&tracing_ctx, method, "Saldo retrieved from cache")
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

        let saldo_result = self.saldo_repository.find_by_user_id(id).await;
        let saldo_opt = match saldo_result {
            Ok(s) => s.map(SaldoResponse::from),
            Err(err) => {
                let msg = format!("Failed to retrieve saldo for user_id {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        match saldo_opt {
            Some(saldo) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "Saldo retrieved successfully".to_string(),
                    data: Some(saldo.clone()),
                };

                self.cache_store
                    .set_to_cache(&cache_key, &response, Duration::from_secs(60 * 5));

                self.complete_tracing_success(
                    &tracing_ctx,
                    method,
                    "Saldo retrieved from database",
                )
                .await;

                Ok(response)
            }
            None => {
                let msg = format!("No saldo found for user_id: {id}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;

                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
        }
    }

    async fn create_saldo(
        &self,
        input: &CreateSaldoRequest,
    ) -> Result<ApiResponse<SaldoResponse>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "CreateSaldo",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("user_id", input.user_id.to_string()),
                KeyValue::new("total_balance", input.total_balance.to_string()),
            ],
        );

        let mut request = Request::new(input.user_id);
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

        let saldo = match self.saldo_repository.create(input).await {
            Ok(saldo) => saldo,
            Err(err) => {
                let msg = format!("Failed to create saldo for user {}: {err}", input.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        let response = ApiResponse {
            status: "success".to_string(),
            message: "Saldo created successfully".to_string(),
            data: SaldoResponse::from(saldo.clone()),
        };

        self.complete_tracing_success(&tracing_ctx, method, "Saldo created successfully")
            .await;

        Ok(response)
    }

    async fn update_saldo(
        &self,
        input: &UpdateSaldoRequest,
    ) -> Result<ApiResponse<SaldoResponse>, ErrorResponse> {
        let method = Method::Put;

        let tracing_ctx = self.start_tracing(
            "UpdateSaldo",
            vec![
                KeyValue::new("component", "saldo"),
                KeyValue::new("user_id", input.user_id.to_string()),
                KeyValue::new("saldo_id", input.saldo_id.to_string()),
                KeyValue::new("total_balance", input.total_balance.to_string()),
            ],
        );

        let _user = match self.user_repository.find_by_id(input.user_id).await {
            Ok(user) => user,
            Err(_) => {
                let msg = format!("User with id {} not found", input.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
        };

        let _existing_saldo = match self.saldo_repository.find_by_id(input.saldo_id).await {
            Ok(Some(saldo)) => saldo,
            Ok(None) => {
                let msg = format!("Saldo with id {} not found", input.saldo_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
            Err(err) => {
                let msg = format!("Error finding saldo with id {}: {}", input.saldo_id, err);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        let updated_saldo = match self.saldo_repository.update(input).await {
            Ok(saldo) => saldo,
            Err(err) => {
                let msg = format!("Failed to update saldo {}: {}", input.saldo_id, err);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        let response = ApiResponse {
            status: "success".to_string(),
            message: "Saldo updated successfully".to_string(),
            data: SaldoResponse::from(updated_saldo),
        };

        self.complete_tracing_success(&tracing_ctx, method, "Saldo updated successfully")
            .await;

        Ok(response)
    }

    async fn delete_saldo(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "DeleteSaldo",
            vec![
                KeyValue::new("component", "saldo"),
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

        let existing_saldo = match self.saldo_repository.find_by_user_id(user.user_id).await {
            Ok(Some(saldo)) => saldo,
            Ok(None) => {
                let msg = format!("Saldo with user_id {} not found", user.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(msg)));
            }
            Err(err) => {
                let msg = format!("Failed to find saldo for user_id {}: {err}", user.user_id);
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                return Err(ErrorResponse::from(err));
            }
        };

        if let Err(err) = self.saldo_repository.delete(existing_saldo.saldo_id).await {
            let msg = format!(
                "Failed to delete saldo id {}: {}",
                existing_saldo.saldo_id, err
            );
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;
            return Err(ErrorResponse::from(err));
        }

        let cache_key = format!("saldo_user:id={}", user.user_id);

        self.cache_store.delete_from_cache(&cache_key);

        info!("Saldo deleted successfully for user_id: {}", user.user_id);

        self.complete_tracing_success(&tracing_ctx, method, "Saldo deleted successfully")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "Saldo deleted successfully".to_string(),
            data: (),
        })
    }
}
