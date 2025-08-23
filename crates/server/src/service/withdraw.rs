use genproto::api::ApiResponseEmpty;
use genproto::withdraw::{
    ApiResponseWithdrawResponse, ApiResponsesWithdrawPaginated, ApiResponsesWithdrawResponse,
    CreateWithdrawRequest, FindAllWithdrawRequest, FindWithdrawByIdRequest,
    FindWithdrawByUserIdRequest, UpdateWithdrawRequest, withdraw_service_server::WithdrawService,
};
use shared::{
    domain::request::{
        CreateWithdrawRequest as SharedCreateWithdrawRequest,
        FindAllWithdrawRequest as SharedFindAllWithdrawRequest,
        UpdateWithdrawRequest as SharedUpdateWithdrawRequest,
    },
    state::AppState,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct WithdrawServiceImpl {
    pub state: Arc<AppState>,
}

impl WithdrawServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl WithdrawService for WithdrawServiceImpl {
    async fn find_all_withdraw(
        &self,
        request: Request<FindAllWithdrawRequest>,
    ) -> Result<Response<ApiResponsesWithdrawPaginated>, Status> {
        info!("Finding all withdraws");

        let req = request.get_ref();

        let body = SharedFindAllWithdrawRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        };

        match self
            .state
            .di_container
            .withdraw_service
            .get_withdraws(&body)
            .await
        {
            Ok(api_response) => {
                let withdraw_responses: Vec<_> =
                    api_response.data.into_iter().map(Into::into).collect();

                info!("Withdraw fetched successfully");

                let reply = ApiResponsesWithdrawPaginated {
                    status: api_response.status,
                    message: api_response.message,
                    data: withdraw_responses,
                    pagination: Some(api_response.pagination.into()),
                };

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Failed to fetch withdraws: {}", err.message);

                Err(Status::internal("Failed to fetch withdraws"))
            }
        }
    }

    async fn find_withdraw_by_id(
        &self,
        request: Request<FindWithdrawByIdRequest>,
    ) -> Result<Response<ApiResponseWithdrawResponse>, Status> {
        info!("Finding withdraw by id");

        let withdraw_id = request.into_inner().id;

        match self
            .state
            .di_container
            .withdraw_service
            .get_withdraw(withdraw_id)
            .await
        {
            Ok(api_response) => match api_response.data {
                Some(data) => {
                    let reply = ApiResponseWithdrawResponse {
                        status: api_response.status,
                        message: api_response.message,
                        data: Some(data.into()),
                    };

                    info!("Withdraw fetched successfully");

                    Ok(Response::new(reply))
                }
                None => Err(Status::not_found("Withdraw not found")),
            },
            Err(err) => {
                error!("Failed to fetch withdraw: {}", err.message);

                Err(Status::internal("Failed to fetch withdraw"))
            }
        }
    }

    async fn find_withdraw_by_user_id(
        &self,
        request: Request<FindWithdrawByUserIdRequest>,
    ) -> Result<Response<ApiResponseWithdrawResponse>, Status> {
        info!("Finding withdraw by user id");

        let user_id = request.into_inner().user_id;

        match self
            .state
            .di_container
            .withdraw_service
            .get_withdraw_user(user_id)
            .await
        {
            Ok(api_response) => match api_response.data {
                Some(data) => {
                    let reply = ApiResponseWithdrawResponse {
                        status: api_response.status,
                        message: api_response.message,
                        data: Some(data.into()),
                    };

                    info!("Withdraw fetched successfully");

                    Ok(Response::new(reply))
                }
                None => Err(Status::not_found("Withdraw not found")),
            },
            Err(err) => {
                error!("Failed to fetch withdraw: {}", err.message);

                Err(Status::internal("Failed to fetch withdraw"))
            }
        }
    }

    async fn find_withdraw_by_users_id(
        &self,
        request: Request<FindWithdrawByUserIdRequest>,
    ) -> Result<Response<ApiResponsesWithdrawResponse>, Status> {
        info!("Finding withdraw by user id");

        let id = request.into_inner().user_id;

        match self
            .state
            .di_container
            .withdraw_service
            .get_withdraw_users(id)
            .await
        {
            Ok(api_response) => {
                let data = api_response
                    .data
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect();

                let reply = ApiResponsesWithdrawResponse {
                    status: "success".into(),
                    message: "Topup fetched successfully".into(),
                    data,
                };

                info!("Topup fetched successfully");

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Failed to fetch topup: {}", err.message);

                Err(Status::internal("Failed to fetch topup"))
            }
        }
    }

    async fn create_withdraw(
        &self,
        request: Request<CreateWithdrawRequest>,
    ) -> Result<Response<ApiResponseWithdrawResponse>, Status> {
        info!("Creating withdraw");

        let req = request.get_ref();

        let body = SharedCreateWithdrawRequest {
            user_id: req.user_id,
            withdraw_amount: req.withdraw_amount,
            withdraw_time: req.withdraw_time.clone(),
        };

        match self
            .state
            .di_container
            .withdraw_service
            .create_withdraw(&body)
            .await
        {
            Ok(api_response) => {
                let reply = ApiResponseWithdrawResponse {
                    status: api_response.status,
                    message: api_response.message,
                    data: Some(api_response.data.into()),
                };

                info!("Withdraw created successfully");

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Failed to create withdraw: {}", err.message);

                Err(Status::internal("Failed to create withdraw"))
            }
        }
    }

    async fn update_withdraw(
        &self,
        request: Request<UpdateWithdrawRequest>,
    ) -> Result<Response<ApiResponseWithdrawResponse>, Status> {
        info!("Updating withdraw");

        let req = request.get_ref();

        let body = SharedUpdateWithdrawRequest {
            user_id: req.user_id,
            withdraw_id: req.withdraw_id,
            withdraw_amount: req.withdraw_amount,
            withdraw_time: req.withdraw_time.clone(),
        };

        match self
            .state
            .di_container
            .withdraw_service
            .update_withdraw(&body)
            .await
        {
            Ok(api_response) => {
                let reply = ApiResponseWithdrawResponse {
                    status: api_response.status,
                    message: api_response.message,
                    data: Some(api_response.data.into()),
                };

                info!("Withdraw updated successfully");

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Failed to update withdraw: {}", err.message);

                Err(Status::internal("Failed to update withdraw"))
            }
        }
    }

    async fn delete_withdraw(
        &self,
        request: Request<FindWithdrawByIdRequest>,
    ) -> Result<Response<ApiResponseEmpty>, Status> {
        info!("Deleting withdraw");

        let withdraw_id = request.into_inner().id;

        match self
            .state
            .di_container
            .withdraw_service
            .delete_withdraw(withdraw_id)
            .await
        {
            Ok(user) => {
                info!("Withdraw deleted successfully");
                Ok(Response::new(ApiResponseEmpty {
                    status: user.status,
                    message: user.message,
                }))
            }
            Err(err) => {
                error!("Failed to delete withdraw: {}", err.message);
                Err(Status::internal("Failed to delete withdraw"))
            }
        }
    }
}
