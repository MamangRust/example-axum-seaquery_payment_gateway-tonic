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
    abstract_trait::{DynHashing, DynUserRepository, UserServiceTrait},
    cache::CacheStore,
    domain::{
        request::{CreateUserRequest, FindAllUserRequest, RegisterRequest, UpdateUserRequest},
        response::{
            ApiResponse, ApiResponsePagination, ErrorResponse, pagination::Pagination,
            user::UserResponse,
        },
    },
    utils::{
        AppError, MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext,
        random_vcc,
    },
};

#[derive(Clone)]
pub struct UserService {
    repository: DynUserRepository,
    hashing: DynHashing,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl std::fmt::Debug for UserService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserService")
            .field("repository", &"DynUserRepository")
            .field("hashing", &"DynHashing")
            .finish()
    }
}

impl UserService {
    pub async fn new(
        repository: DynUserRepository,
        hashing: DynHashing,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.register(
            "user_service_request_counter",
            "Total number of requests to the UserService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "user_service_request_duration",
            "Histogram of request> durations for the UserService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            repository,
            hashing,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("user-service")
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
        req: &FindAllUserRequest,
    ) -> Result<ApiResponsePagination<Vec<UserResponse>>, ErrorResponse> {
        let method = Method::Get;

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 { req.page_size } else { 10 };
        let search = if req.search.is_empty() {
            None
        } else {
            Some(req.search.clone())
        };

        let tracing_ctx = self.start_tracing(
            "Getusers",
            vec![
                KeyValue::new("component", "category"),
                KeyValue::new("page", page.to_string()),
                KeyValue::new("page_size", page_size.to_string()),
                KeyValue::new("search", search.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(FindAllUserRequest {
            page,
            page_size,
            search: search.clone().unwrap_or_default(),
        });

        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!(
            "users:page={page}:size={page_size}:search={}",
            search.clone().unwrap_or_default()
        );

        if let Some(cached) = self
            .cache_store
            .get_from_cache::<ApiResponsePagination<Vec<UserResponse>>>(&cache_key)
        {
            info!("Found users in cache");

            self.complete_tracing_success(&tracing_ctx, method, "users retrieved from cache")
                .await;

            return Ok(cached);
        }

        match self.repository.find_all(page, page_size, search).await {
            Ok((users, total_items)) => {
                let total_pages = (total_items as f64 / page_size as f64).ceil() as i32;
                let users_responses = users
                    .into_iter()
                    .map(UserResponse::from)
                    .collect::<Vec<_>>();

                let response = ApiResponsePagination {
                    status: "success".to_string(),
                    message: "users retrieved successfully".to_string(),
                    data: users_responses.clone(),
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
                    "users retrieved from database",
                )
                .await;

                Ok(response)
            }

            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to retrieve users: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
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

        match self.repository.find_by_id(id).await {
            Ok(Some(user)) => {
                self.complete_tracing_success(&tracing_ctx, method, "User retrieved successfully")
                    .await;

                Ok(ApiResponse {
                    status: "success".to_string(),
                    message: "User retrieved successfully".to_string(),
                    data: Some(UserResponse::from(user)),
                })
            }
            Ok(None) => {
                let msg = format!("User with id {id} not found");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                Err(ErrorResponse::from(AppError::NotFound(msg)))
            }
            Err(err) => {
                let msg = format!("Failed to retrieve user {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn create_user(
        &self,
        input: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "CreateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("email", input.email.clone()),
            ],
        );

        let mut request = Request::new(input.email.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        info!("Attempting to register user with email: {}", input.email);

        let exists = self.repository.find_by_email_exists(&input.email).await?;

        if exists {
            let msg = format!("Email already exists: {}", input.email);
            error!("{}", msg);
            self.complete_tracing_error(&tracing_ctx, method, &msg)
                .await;
            return Err(ErrorResponse::from(AppError::EmailAlreadyExists));
        }

        let hashed_password = self.hashing.hash_password(&input.password).await;

        let hashed_password = match hashed_password {
            Ok(val) => val,
            Err(e) => {
                let msg = format!("Failed to hash password for email {}: {}", input.email, e);
                error!("{msg}");

                self.complete_tracing_error(&tracing_ctx, method.clone(), &msg)
                    .await;

                return Err(AppError::HashingError(e).into());
            }
        };

        let noc_transfer = random_vcc().map(Some).unwrap_or(None);

        let request = &CreateUserRequest {
            firstname: input.firstname.clone(),
            lastname: input.lastname.clone(),
            email: input.email.clone(),
            password: hashed_password,
            confirm_password: input.confirm_password.clone(),
            noc_transfer: noc_transfer.to_owned(),
        };

        info!("Creating user with email: {}", input.email);
        let create_user = self.repository.create_user(request).await?;

        info!("User created successfully with email: {}", input.email);
        self.complete_tracing_success(&tracing_ctx, method.clone(), "User created successfully")
            .await;

        Ok(ApiResponse {
            status: "success".to_string(),
            message: "User created successfully".to_string(),
            data: UserResponse::from(create_user),
        })
    }

    async fn update_user(
        &self,
        input: &UpdateUserRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        let method = Method::Put;
        let tracing_ctx = self.start_tracing(
            "UpdateUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("user.email", input.email.clone().unwrap_or_default()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        match self.repository.update_user(input).await {
            Ok(user) => {
                let user_id = user.clone().user_id;

                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "User updated successfully".to_string(),
                    data: UserResponse::from(user),
                };

                self.cache_store.set_to_cache(
                    &format!("user:id={user_id}"),
                    &response,
                    Duration::from_secs(60 * 5),
                );

                self.complete_tracing_success(&tracing_ctx, method, "User updated successfully")
                    .await;

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Failed to update user: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn delete_user(&self, id: i32) -> Result<ApiResponse<()>, ErrorResponse> {
        let method = Method::Delete;

        let tracing_ctx = self.start_tracing(
            "DeleteUser",
            vec![
                KeyValue::new("component", "user"),
                KeyValue::new("user_id", id.to_string()),
            ],
        );

        match self.repository.delete_user(id).await {
            Ok(_) => {
                self.complete_tracing_success(&tracing_ctx, method, "User deleted successfully")
                    .await;

                Ok(ApiResponse {
                    status: "success".to_string(),
                    message: "User deleted successfully".to_string(),
                    data: (),
                })
            }
            Err(err) => {
                let msg = format!("Failed to delete user {id}: {err}");
                self.complete_tracing_error(&tracing_ctx, method, &msg)
                    .await;
                Err(ErrorResponse::from(err))
            }
        }
    }
}
