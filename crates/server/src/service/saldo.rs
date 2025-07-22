use genproto::api::ApiResponseEmpty;
use genproto::saldo::{
    ApiResponseSaldoResponse, ApiResponsesSaldoPaginated, ApiResponsesSaldoResponse,
    CreateSaldoRequest, FindAllSaldoRequest, FindSaldoByIdRequest, FindSaldoByUserIdRequest,
    UpdateSaldoRequest, saldo_service_server::SaldoService,
};
use shared::{
    domain::request::{
        CreateSaldoRequest as SharedCreateSaldoRequest,
        FindAllSaldoRequest as SharedFindAllSaldoRequest,
        UpdateSaldoRequest as SharedUpdateSaldoRequest,
    },
    state::AppState,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct SaldoServiceImpl {
    state: Arc<AppState>,
}

impl SaldoServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl SaldoService for SaldoServiceImpl {
    async fn find_all_saldo(
        &self,
        request: Request<FindAllSaldoRequest>,
    ) -> Result<Response<ApiResponsesSaldoPaginated>, Status> {
        let req = request.get_ref();

        let my_request = SharedFindAllSaldoRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        };

        match self
            .state
            .di_container
            .saldo_service
            .get_saldos(&my_request)
            .await
        {
            Ok(api_response) => {
                let saldo_responses: Vec<_> =
                    api_response.data.into_iter().map(Into::into).collect();

                Ok(Response::new(ApiResponsesSaldoPaginated {
                    status: api_response.status,
                    message: api_response.message,
                    data: saldo_responses,
                    pagination: Some(api_response.pagination.into()),
                }))
            }
            Err(err) => {
                tracing::error!("Failed to fetch saldo: {}", err);
                Err(Status::internal("Failed to fetch saldo"))
            }
        }
    }

    async fn find_saldo_by_id(
        &self,
        request: Request<FindSaldoByIdRequest>,
    ) -> Result<Response<ApiResponseSaldoResponse>, Status> {
        let id = request.into_inner().id;

        match self.state.di_container.saldo_service.get_saldo(id).await {
            Ok(api_response) => match api_response.data {
                Some(saldo) => {
                    let reply = ApiResponseSaldoResponse {
                        status: "success".into(),
                        message: "Saldo fetched successfully".into(),
                        data: Some(saldo.into()),
                    };
                    Ok(Response::new(reply))
                }
                None => Err(Status::not_found("Saldo not found")),
            },
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn find_saldo_by_user_id(
        &self,
        request: Request<FindSaldoByUserIdRequest>,
    ) -> Result<Response<ApiResponseSaldoResponse>, Status> {
        let user_id = request.into_inner().user_id;

        match self
            .state
            .di_container
            .saldo_service
            .get_saldo_user(user_id)
            .await
        {
            Ok(api_response) => match api_response.data {
                Some(saldo) => {
                    let reply = ApiResponseSaldoResponse {
                        status: "success".into(),
                        message: "Saldo fetched successfully".into(),
                        data: Some(saldo.into()),
                    };
                    Ok(Response::new(reply))
                }
                None => Err(Status::not_found("Saldo not found")),
            },
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn find_saldo_by_users_id(
        &self,
        request: Request<FindSaldoByUserIdRequest>,
    ) -> Result<Response<ApiResponsesSaldoResponse>, Status> {
        let user_id = request.into_inner().user_id;

        match self
            .state
            .di_container
            .saldo_service
            .get_saldo_users(user_id)
            .await
        {
            Ok(api_response) => {
                let data_vec = api_response
                    .data
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect();

                let reply = ApiResponsesSaldoResponse {
                    status: "success".into(),
                    message: "Saldo fetched successfully".into(),
                    data: data_vec,
                };

                Ok(Response::new(reply))
            }
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn create_saldo(
        &self,
        request: Request<CreateSaldoRequest>,
    ) -> Result<Response<ApiResponseSaldoResponse>, Status> {
        let req = request.get_ref();

        let body = SharedCreateSaldoRequest {
            user_id: req.user_id,
            total_balance: req.total_balance,
        };

        match self
            .state
            .di_container
            .saldo_service
            .create_saldo(&body)
            .await
        {
            Ok(api_response) => Ok(Response::new(ApiResponseSaldoResponse {
                status: api_response.status,
                message: api_response.message,
                data: Some(api_response.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn update_saldo(
        &self,
        request: Request<UpdateSaldoRequest>,
    ) -> Result<Response<ApiResponseSaldoResponse>, Status> {
        let req = request.get_ref();

        let body = SharedUpdateSaldoRequest {
            saldo_id: req.user_id,
            user_id: req.user_id,
            total_balance: req.total_balance,
            withdraw_amount: None,
            withdraw_time: None,
        };

        match self
            .state
            .di_container
            .saldo_service
            .update_saldo(&body)
            .await
        {
            Ok(api_response) => Ok(Response::new(ApiResponseSaldoResponse {
                status: api_response.status,
                message: api_response.message,
                data: Some(api_response.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn delete_saldo(
        &self,
        request: Request<FindSaldoByIdRequest>,
    ) -> Result<Response<ApiResponseEmpty>, Status> {
        let id = request.into_inner().id;

        match self.state.di_container.saldo_service.delete_saldo(id).await {
            Ok(user) => Ok(Response::new(ApiResponseEmpty {
                status: user.status,
                message: user.message,
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }
}
