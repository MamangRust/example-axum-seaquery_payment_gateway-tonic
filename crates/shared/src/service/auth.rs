use crate::{
    abstract_trait::{AuthServiceTrait, DynHashing, DynJwtService, DynUserRepository},
    cache::CacheStore,
    domain::{
        request::{CreateUserRequest, LoginRequest, RegisterRequest},
        response::{ApiResponse, ErrorResponse, user::UserResponse},
    },
    utils::{
        AppError, MetadataInjector, Method, Metrics, Status as StatusUtils, TracingContext,
        random_vcc,
    },
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

#[derive(Clone)]
pub struct AuthService {
    repository: DynUserRepository,
    hashing: DynHashing,
    jwt_config: DynJwtService,
    metrics: Arc<Mutex<Metrics>>,
    cache_store: Arc<CacheStore>,
}

impl std::fmt::Debug for AuthService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthService")
            .field("repository", &"DynUserRepository")
            .field("hashing", &"Hashing")
            .field("jwt_config", &"JwtConfig")
            .finish()
    }
}

impl AuthService {
    pub async fn new(
        repository: DynUserRepository,
        hashing: DynHashing,
        jwt_config: DynJwtService,
        metrics: Arc<Mutex<Metrics>>,
        registry: &mut Registry,
        cache_store: Arc<CacheStore>,
    ) -> Self {
        registry.register(
            "auth_service_request_counter",
            "Total number of requests to the AuthService",
            metrics.lock().await.request_counter.clone(),
        );
        registry.register(
            "auth_service_request_duration",
            "Histogram of request durations for the AuthService",
            metrics.lock().await.request_duration.clone(),
        );

        Self {
            repository,
            hashing,
            jwt_config,
            metrics,
            cache_store,
        }
    }

    fn get_tracer(&self) -> BoxedTracer {
        global::tracer("auth-service")
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

        info!("Starting operation: {}", operation_name);

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
            info!("Operation completed successfully: {}", message);
        } else {
            error!("Operation failed: {}", message);
        }

        self.metrics.lock().await.record(method, status, elapsed);

        tracing_ctx.cx.span().end();
    }
}

#[async_trait]
impl AuthServiceTrait for AuthService {
    async fn register_user(
        &self,
        input: &RegisterRequest,
    ) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        let method = Method::Post;
        let tracing_ctx = self.start_tracing(
            "RegisterUser",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.email", input.email.clone()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("auth:registered:{}", input.email);

        if let Some(cached_user) = self.cache_store.get_from_cache(&cache_key) {
            info!("Found user in cache");

            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "User already registered (from cache)",
            )
            .await;

            return Ok(ApiResponse {
                status: "success".to_string(),
                message: "User already registered (from cache)".to_string(),
                data: cached_user,
            });
        }

        match self.repository.find_by_email_exists(&input.email).await {
            Ok(true) => {
                self.complete_tracing_error(&tracing_ctx, method, "Email already exists")
                    .await;
                return Err(ErrorResponse::from(AppError::EmailAlreadyExists));
            }
            Ok(false) => (),
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Error checking email: {err}"),
                )
                .await;
                return Err(ErrorResponse::from(err));
            }
        }

        let hashed_password = match self.hashing.hash_password(&input.password).await {
            Ok(hashed) => hashed,
            Err(e) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Password hashing failed: {e}"),
                )
                .await;
                return Err(ErrorResponse::from(AppError::HashingError(e)));
            }
        };

        let noc_transfer = random_vcc().ok();

        let create_user_request = CreateUserRequest {
            firstname: input.firstname.clone(),
            lastname: input.lastname.clone(),
            email: input.email.clone(),
            password: hashed_password,
            confirm_password: input.confirm_password.clone(),
            noc_transfer: noc_transfer.to_owned(),
        };

        match self.repository.create_user(&create_user_request).await {
            Ok(user) => {
                let response = ApiResponse {
                    status: "success".to_string(),
                    message: "User registered successfully".to_string(),
                    data: UserResponse::from(user),
                };

                self.complete_tracing_success(&tracing_ctx, method, "User registered successfully")
                    .await;

                self.cache_store.set_to_cache(
                    &cache_key,
                    &response.data.clone(),
                    Duration::from_secs(60),
                );

                Ok(response)
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("User registration failed: {err}"),
                )
                .await;

                Err(ErrorResponse::from(err))
            }
        }
    }

    async fn login_user(&self, input: &LoginRequest) -> Result<ApiResponse<String>, ErrorResponse> {
        let method = Method::Post;

        let tracing_ctx = self.start_tracing(
            "LoginUser",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.email", input.email.clone()),
            ],
        );

        let mut request = Request::new(input.clone());
        self.inject_trace_context(&tracing_ctx.cx, &mut request);

        let cache_key = format!("auth:login:{}", input.email);

        if let Some(cached_token) = self.cache_store.get_from_cache(&cache_key) {
            info!("Found token in cache");

            self.complete_tracing_success(
                &tracing_ctx,
                method,
                "User already logged in (from cache)",
            )
            .await;

            return Ok(ApiResponse {
                status: "success".to_string(),
                message: "User already logged in (from cache)".to_string(),
                data: cached_token,
            });
        }

        let user = match self.repository.find_by_email(&input.email).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                self.complete_tracing_error(&tracing_ctx, method, "User not found")
                    .await;
                return Err(ErrorResponse::from(AppError::NotFound(
                    "User not found".to_string(),
                )));
            }
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Error finding user: {err}"),
                )
                .await;
                return Err(ErrorResponse::from(err));
            }
        };

        if (self
            .hashing
            .compare_password(&user.password, &input.password)
            .await)
            .is_err()
        {
            self.complete_tracing_error(&tracing_ctx, method, "Invalid credentials")
                .await;
            return Err(ErrorResponse::from(AppError::InvalidCredentials));
        }

        let token = match self.jwt_config.generate_token(user.user_id as i64) {
            Ok(token) => token,
            Err(err) => {
                self.complete_tracing_error(
                    &tracing_ctx,
                    method,
                    &format!("Token generation failed: {err}"),
                )
                .await;
                return Err(ErrorResponse::from(err));
            }
        };

        self.cache_store
            .set_to_cache(&input.email, &token, Duration::from_secs(60));

        let response = ApiResponse {
            status: "success".to_string(),
            message: "Login successful".to_string(),
            data: token,
        };

        self.complete_tracing_success(&tracing_ctx, method, "Login successful")
            .await;

        Ok(response)
    }

    async fn get_me(&self, id: i32) -> Result<ApiResponse<UserResponse>, ErrorResponse> {
        let method = Method::Get;

        let tracing_ctx = self.start_tracing(
            "GetMe",
            vec![
                KeyValue::new("component", "auth"),
                KeyValue::new("user.id", id.to_string()),
            ],
        );

        match self.repository.find_by_id(id).await {
            Ok(Some(user)) => {
                self.complete_tracing_success(&tracing_ctx, method, "User retrieved successfully")
                    .await;

                Ok(ApiResponse {
                    status: "success".to_string(),
                    message: "User retrieved successfully".to_string(),
                    data: UserResponse::from(user),
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
}
