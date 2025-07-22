use std::sync::Arc;
use tonic::{Request, Response, Status};

use genproto::auth::{
    ApiResponseGetMe, ApiResponseLogin, ApiResponseRegister, GetMeRequest, LoginRequest,
    RegisterRequest, auth_service_server::AuthService,
};

use shared::{
    domain::request::{
        LoginRequest as LoginDomainRequest, RegisterRequest as RegisterDomainRequest,
    },
    state::AppState,
};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct AuthServiceImpl {
    pub state: Arc<AppState>,
}

impl AuthServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn login_user(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<ApiResponseLogin>, Status> {
        info!("Logging in user {}", request.get_ref().email);

        let req = request.into_inner();

        let domain_req = LoginDomainRequest {
            email: req.email,
            password: req.password,
        };

        match self
            .state
            .di_container
            .auth_service
            .login_user(&domain_req)
            .await
        {
            Ok(api_response) => {
                let reply = ApiResponseLogin {
                    status: api_response.status,
                    message: api_response.message,
                    data: api_response.data,
                };

                info!("User logged in successfully");

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Error logging in user: {}", err.message);
                Err(Status::internal(err.message))
            }
        }
    }

    async fn register_user(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<ApiResponseRegister>, Status> {
        info!("Registering user {}", request.get_ref().email);

        let req = request.into_inner();

        let domain_req = RegisterDomainRequest {
            firstname: req.firstname,
            lastname: req.lastname,
            email: req.email,
            password: req.password,
            confirm_password: req.confirm_password,
        };

        match self
            .state
            .di_container
            .auth_service
            .register_user(&domain_req)
            .await
        {
            Ok(api_response) => {
                let user = Some(api_response.data.into());
                let reply = ApiResponseRegister {
                    status: api_response.status,
                    message: api_response.message,
                    data: user,
                };

                info!("User registered successfully");

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Error registering user: {}", err.message);
                Err(Status::internal(err.message))
            }
        }
    }

    async fn get_me(
        &self,
        request: Request<GetMeRequest>,
    ) -> Result<Response<ApiResponseGetMe>, Status> {
        info!("Getting user profile");

        let req = request.into_inner();

        match self.state.di_container.auth_service.get_me(req.id).await {
            Ok(api_response) => {
                let reply = ApiResponseGetMe {
                    status: "success".into(),
                    message: "User fetched successfully".into(),
                    data: Some(api_response.data.into()),
                };

                info!("User fetched successfully");

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Error fetching user: {}", err.message);
                Err(Status::internal(err.message))
            }
        }
    }
}
