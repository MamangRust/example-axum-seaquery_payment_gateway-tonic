use genproto::api::ApiResponseEmpty;
use genproto::topup::{
    ApiResponseTopupResponse, ApiResponsesTopupPaginated, ApiResponsesTopupResponse,
    CreateTopupRequest, FindAllTopupRequest, FindTopupByIdRequest, FindTopupByUserIdRequest,
    UpdateTopupRequest, topup_service_server::TopupService,
};
use shared::{
    domain::request::{
        CreateTopupRequest as SharedCreateTopupRequest,
        FindAllTopupRequest as SharedFindAllTopupRequest,
        UpdateTopupRequest as SharedUpdateTopupRequest,
    },
    state::AppState,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct TopupServiceImpl {
    pub state: Arc<AppState>,
}

impl TopupServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl TopupService for TopupServiceImpl {
    async fn find_all_topup(
        &self,
        request: Request<FindAllTopupRequest>,
    ) -> Result<Response<ApiResponsesTopupPaginated>, Status> {
        let req = request.get_ref();

        let my_request = SharedFindAllTopupRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        };

        match self
            .state
            .di_container
            .topup_service
            .get_topups(&my_request)
            .await
        {
            Ok(api_response) => {
                let topup_responses: Vec<_> =
                    api_response.data.into_iter().map(Into::into).collect();

                Ok(Response::new(ApiResponsesTopupPaginated {
                    status: api_response.status,
                    message: api_response.message,
                    data: topup_responses,
                    pagination: Some(api_response.pagination.into()),
                }))
            }
            Err(err) => {
                tracing::error!("Failed to fetch topups: {}", err);
                Err(Status::internal("Failed to fetch topups"))
            }
        }
    }

    async fn find_topup_by_id(
        &self,
        request: Request<FindTopupByIdRequest>,
    ) -> Result<Response<ApiResponseTopupResponse>, Status> {
        let id = request.into_inner().id;

        match self.state.di_container.topup_service.get_topup(id).await {
            Ok(api_response) => match api_response.data {
                Some(topup) => {
                    let reply = ApiResponseTopupResponse {
                        status: "success".into(),
                        message: "Topup fetched successfully".into(),
                        data: Some(topup.into()),
                    };
                    Ok(Response::new(reply))
                }
                None => Err(Status::not_found("Topup not found")),
            },
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn find_topup_by_user_id(
        &self,
        request: Request<FindTopupByUserIdRequest>,
    ) -> Result<Response<ApiResponseTopupResponse>, Status> {
        let user_id = request.into_inner().user_id;

        match self
            .state
            .di_container
            .topup_service
            .get_topup_user(user_id)
            .await
        {
            Ok(api_response) => match api_response.data {
                Some(topup) => {
                    let reply = ApiResponseTopupResponse {
                        status: "success".into(),
                        message: "Topup fetched successfully".into(),
                        data: Some(topup.into()),
                    };
                    Ok(Response::new(reply))
                }
                None => Err(Status::not_found("Topup not found")),
            },
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn find_topup_by_users_id(
        &self,
        request: Request<FindTopupByUserIdRequest>,
    ) -> Result<Response<ApiResponsesTopupResponse>, Status> {
        match self
            .state
            .di_container
            .topup_service
            .get_topup_users(request.into_inner().user_id)
            .await
        {
            Ok(api_response) => {
                let data = api_response
                    .data
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect();

                let reply = ApiResponsesTopupResponse {
                    status: "success".into(),
                    message: "Topup fetched successfully".into(),
                    data,
                };

                Ok(Response::new(reply))
            }
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn create_topup(
        &self,
        request: Request<CreateTopupRequest>,
    ) -> Result<Response<ApiResponseTopupResponse>, Status> {
        let req = request.get_ref();

        let body = SharedCreateTopupRequest {
            user_id: req.user_id,
            topup_no: req.topup_no.to_string(),
            topup_amount: req.topup_amount,
            topup_method: req.topup_method.to_string(),
        };

        match self
            .state
            .di_container
            .topup_service
            .create_topup(&body)
            .await
        {
            Ok(api_response) => Ok(Response::new(ApiResponseTopupResponse {
                status: api_response.status,
                message: api_response.message,
                data: Some(api_response.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn update_topup(
        &self,
        request: Request<UpdateTopupRequest>,
    ) -> Result<Response<ApiResponseTopupResponse>, Status> {
        let req = request.get_ref();

        let body = SharedUpdateTopupRequest {
            user_id: req.user_id,
            topup_id: req.topup_id,
            topup_amount: req.topup_amount,
            topup_method: req.topup_method.to_string(),
        };

        match self
            .state
            .di_container
            .topup_service
            .update_topup(&body)
            .await
        {
            Ok(api_response) => Ok(Response::new(ApiResponseTopupResponse {
                status: api_response.status,
                message: api_response.message,
                data: Some(api_response.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn delete_topup(
        &self,
        request: Request<FindTopupByIdRequest>,
    ) -> Result<Response<ApiResponseEmpty>, Status> {
        let id = request.into_inner().id;

        match self.state.di_container.topup_service.delete_topup(id).await {
            Ok(user) => Ok(Response::new(ApiResponseEmpty {
                status: user.status,
                message: user.message,
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }
}
