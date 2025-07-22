use genproto::api::ApiResponseEmpty;
use genproto::transfer::{
    ApiResponseTransferResponse, ApiResponsesTransferPaginated, ApiResponsesTransferResponse,
    CreateTransferRequest, FindAllTransferRequest, FindTransferByIdRequest,
    FindTransferByUserIdRequest, UpdateTransferRequest, transfer_service_server::TransferService,
};
use shared::{
    domain::request::{
        CreateTransferRequest as SharedCreateTransferRequest,
        FindAllTransferRequest as SharedFindAllTransferRequest,
        UpdateTransferRequest as SharedUpdateTransferRequest,
    },
    state::AppState,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub struct TransferServiceImpl {
    pub state: Arc<AppState>,
}

impl TransferServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl TransferService for TransferServiceImpl {
    async fn find_all_transfer(
        &self,
        request: Request<FindAllTransferRequest>,
    ) -> Result<Response<ApiResponsesTransferPaginated>, Status> {
        let req = request.get_ref();

        let body = SharedFindAllTransferRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        };

        match self
            .state
            .di_container
            .transfer_service
            .get_transfers(&body)
            .await
        {
            Ok(api_response) => {
                let transfer_responses: Vec<_> =
                    api_response.data.into_iter().map(Into::into).collect();

                info!("Transfer fetched successfully");

                Ok(Response::new(ApiResponsesTransferPaginated {
                    status: api_response.status,
                    message: api_response.message,
                    data: transfer_responses,
                    pagination: Some(api_response.pagination.into()),
                }))
            }
            Err(err) => {
                error!("Failed to fetch transfer: {}", err.message);
                Err(Status::internal(err.message))
            }
        }
    }

    async fn find_transfer_by_id(
        &self,
        request: Request<FindTransferByIdRequest>,
    ) -> Result<Response<ApiResponseTransferResponse>, Status> {
        let id = request.into_inner().id;

        info!("Finding transfer by id: {}", id);

        match self
            .state
            .di_container
            .transfer_service
            .get_transfer(id)
            .await
        {
            Ok(api_response) => match api_response.data {
                Some(transfer) => {
                    let reply = ApiResponseTransferResponse {
                        status: "success".into(),
                        message: "Transfer fetched successfully".into(),
                        data: Some(transfer.into()),
                    };

                    info!("Transfer fetched successfully");

                    Ok(Response::new(reply))
                }
                None => Err(Status::not_found("Transfer not found")),
            },
            Err(err) => {
                error!("Failed to fetch transfer: {}", err.message);

                Err(Status::internal("Failed to fetch transfer"))
            }
        }
    }

    async fn find_transfer_by_user_id(
        &self,
        request: Request<FindTransferByUserIdRequest>,
    ) -> Result<Response<ApiResponseTransferResponse>, Status> {
        let user_id = request.into_inner().user_id;

        info!("Finding transfer by user id: {}", user_id);

        match self
            .state
            .di_container
            .transfer_service
            .get_transfer_user(user_id)
            .await
        {
            Ok(api_response) => match api_response.data {
                Some(transfer) => {
                    let reply = ApiResponseTransferResponse {
                        status: "success".into(),
                        message: "Transfer fetched successfully".into(),
                        data: Some(transfer.into()),
                    };

                    info!("Transfer fetched successfully");

                    Ok(Response::new(reply))
                }
                None => Err(Status::not_found("Transfer not found")),
            },
            Err(err) => {
                error!("Failed to fetch transfer: {}", err.message);

                Err(Status::internal("Failed to fetch transfer"))
            }
        }
    }

    async fn find_transfer_by_users_id(
        &self,
        request: Request<FindTransferByUserIdRequest>,
    ) -> Result<Response<ApiResponsesTransferResponse>, Status> {
        let user_id = request.into_inner().user_id;

        info!("Finding transfer by user id : {}", user_id);

        match self
            .state
            .di_container
            .transfer_service
            .get_transfer_users(user_id)
            .await
        {
            Ok(api_response) => {
                let data_vec = api_response
                    .data
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect();

                let reply = ApiResponsesTransferResponse {
                    status: "success".into(),
                    message: "Transfer fetched successfully".into(),
                    data: data_vec,
                };

                info!("Transfer fetched successfully");

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Failed to fetch transfer: {}", err.message);

                Err(Status::internal("Failed to fetch transfer"))
            }
        }
    }

    async fn create_transfer(
        &self,
        request: Request<CreateTransferRequest>,
    ) -> Result<Response<ApiResponseTransferResponse>, Status> {
        info!("Creating transfer");

        let req = request.get_ref();

        let body = SharedCreateTransferRequest {
            transfer_from: req.transfer_from,
            transfer_to: req.transfer_to,
            transfer_amount: req.transfer_amount,
        };

        match self
            .state
            .di_container
            .transfer_service
            .create_transfer(&body)
            .await
        {
            Ok(api_response) => {
                let reply = ApiResponseTransferResponse {
                    status: api_response.status,
                    message: api_response.message,
                    data: Some(api_response.data.into()),
                };

                info!("Transfer created successfully");

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Failed to create transfer: {}", err.message);

                Err(Status::internal("Failed to create transfer"))
            }
        }
    }

    async fn update_transfer(
        &self,
        request: Request<UpdateTransferRequest>,
    ) -> Result<Response<ApiResponseTransferResponse>, Status> {
        info!("Updating transfer");

        let req = request.get_ref();

        let body = SharedUpdateTransferRequest {
            transfer_id: req.transfer_id,
            transfer_from: req.transfer_from,
            transfer_to: req.transfer_to,
            transfer_amount: req.transfer_amount,
        };

        match self
            .state
            .di_container
            .transfer_service
            .update_transfer(&body)
            .await
        {
            Ok(api_response) => {
                let reply = ApiResponseTransferResponse {
                    status: api_response.status,
                    message: api_response.message,
                    data: Some(api_response.data.into()),
                };

                info!("Transfer updated successfully");

                Ok(Response::new(reply))
            }
            Err(err) => {
                error!("Failed to update transfer: {}", err.message);

                Err(Status::internal("Failed to update transfer"))
            }
        }
    }

    async fn delete_transfer(
        &self,
        request: Request<FindTransferByIdRequest>,
    ) -> Result<Response<ApiResponseEmpty>, Status> {
        let id = request.into_inner().id;

        match self
            .state
            .di_container
            .transfer_service
            .delete_transfer(id)
            .await
        {
            Ok(_user) => {
                info!("Transfer deleted successfully");

                Ok(Response::new(ApiResponseEmpty {
                    status: "success".into(),
                    message: "Transfer deleted successfully".into(),
                }))
            }
            Err(err) => {
                error!("Failed to delete transfer: {}", err.message);

                Err(Status::internal("Failed to delete transfer"))
            }
        }
    }
}
