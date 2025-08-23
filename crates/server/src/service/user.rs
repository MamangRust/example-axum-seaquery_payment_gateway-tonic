use genproto::api::ApiResponseEmpty;
use genproto::user::{
    ApiResponseUserResponse, ApiResponsesUserPaginated, CreateUserRequest, FindAllUserRequest,
    FindUserByIdRequest, UpdateUserRequest, user_service_server::UserService,
};
use shared::{
    domain::request::{
        FindAllUserRequest as SharedFindAllUserRequest, RegisterRequest,
        UpdateUserRequest as SharedUpdateUserRequest,
    },
    state::AppState,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct UserServiceImpl {
    pub state: Arc<AppState>,
}

impl UserServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl UserService for UserServiceImpl {
    async fn find_all_users(
        &self,
        request: Request<FindAllUserRequest>,
    ) -> Result<Response<ApiResponsesUserPaginated>, Status> {
        info!("Finding all users");

        let req = request.get_ref();

        let myrequest = SharedFindAllUserRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        };

        match self
            .state
            .di_container
            .user_service
            .get_users(&myrequest)
            .await
        {
            Ok(api_response) => {
                let user_responses: Vec<_> =
                    api_response.data.into_iter().map(Into::into).collect();

                info!("User fetched successfully");

                Ok(Response::new(ApiResponsesUserPaginated {
                    status: api_response.status,
                    message: api_response.message,
                    data: user_responses,
                    pagination: Some(api_response.pagination.into()),
                }))
            }
            Err(err) => {
                error!("Failed to fetch users: {}", err.message);
                Err(Status::internal("Failed to fetch users"))
            }
        }
    }

    async fn find_by_id(
        &self,
        request: Request<FindUserByIdRequest>,
    ) -> Result<Response<ApiResponseUserResponse>, Status> {
        info!("Finding user by id");

        let id = request.into_inner().id;

        match self.state.di_container.user_service.get_user(id).await {
            Ok(api_response) => match api_response.data {
                Some(user) => {
                    let reply = ApiResponseUserResponse {
                        status: "success".into(),
                        message: "User fetched successfully".into(),
                        data: Some(user.into()),
                    };

                    info!("User fetched successfully");

                    Ok(Response::new(reply))
                }
                None => Err(Status::not_found("User not found")),
            },
            Err(err) => {
                error!("Failed to fetch user: {}", err.message);
                Err(Status::internal("Failed to fetch user"))
            }
        }
    }

    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<ApiResponseUserResponse>, Status> {
        info!("Creating user");

        let req = request.get_ref();

        let myrequest = RegisterRequest {
            firstname: req.firstname.clone(),
            lastname: req.lastname.clone(),
            email: req.email.clone(),
            password: req.password.clone(),
            confirm_password: req.confirm_password.clone(),
        };

        match self
            .state
            .di_container
            .user_service
            .create_user(&myrequest)
            .await
        {
            Ok(user) => {
                info!("User created successfully");
                Ok(Response::new(ApiResponseUserResponse {
                    status: user.status,
                    message: user.message,
                    data: Some(user.data.into()),
                }))
            }
            Err(err) => {
                error!("Failed to create user: {}", err.message);
                Err(Status::internal("Failed to create user"))
            }
        }
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<ApiResponseUserResponse>, Status> {
        info!("Updating user");

        let req = request.get_ref();

        let body = SharedUpdateUserRequest {
            id: req.id,
            firstname: Some(req.firstname.clone()),
            lastname: Some(req.lastname.clone()),
            email: Some(req.email.clone()),
            password: req.password.clone(),
            confirm_password: req.confirm_password.clone(),
        };

        match self
            .state
            .di_container
            .user_service
            .update_user(&body)
            .await
        {
            Ok(user) => {
                info!("User updated successfully");
                Ok(Response::new(ApiResponseUserResponse {
                    status: user.status,
                    message: user.message,
                    data: Some(user.data.into()),
                }))
            }
            Err(err) => {
                error!("Failed to update user: {}", err.message);
                Err(Status::internal("Failed to update user"))
            }
        }
    }

    async fn delete_user(
        &self,
        request: Request<FindUserByIdRequest>,
    ) -> Result<Response<ApiResponseEmpty>, Status> {
        info!("Deleting user");

        let id = request.into_inner().id;

        match self.state.di_container.user_service.delete_user(id).await {
            Ok(user) => {
                info!("User deleted successfully");
                Ok(Response::new(ApiResponseEmpty {
                    status: user.status,
                    message: user.message,
                }))
            }
            Err(err) => {
                error!("Failed to delete user: {}", err.message);
                Err(Status::internal("Failed to delete user"))
            }
        }
    }
}
