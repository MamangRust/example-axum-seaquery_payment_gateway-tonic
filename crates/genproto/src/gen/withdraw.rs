// This file is @generated by prost-build.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FindAllWithdrawRequest {
    #[prost(int32, tag = "1")]
    pub page: i32,
    #[prost(int32, tag = "2")]
    pub page_size: i32,
    #[prost(string, tag = "3")]
    pub search: ::prost::alloc::string::String,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct FindWithdrawByIdRequest {
    #[prost(int32, tag = "1")]
    pub id: i32,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct FindWithdrawByUserIdRequest {
    #[prost(int32, tag = "1")]
    pub user_id: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateWithdrawRequest {
    #[prost(int32, tag = "1")]
    pub user_id: i32,
    #[prost(int32, tag = "2")]
    pub withdraw_amount: i32,
    #[prost(string, tag = "3")]
    pub withdraw_time: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateWithdrawRequest {
    #[prost(int32, tag = "1")]
    pub withdraw_id: i32,
    #[prost(int32, tag = "2")]
    pub user_id: i32,
    #[prost(int32, tag = "3")]
    pub withdraw_amount: i32,
    #[prost(string, tag = "4")]
    pub withdraw_time: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WithdrawResponse {
    #[prost(int32, tag = "1")]
    pub withdraw_id: i32,
    #[prost(int32, tag = "2")]
    pub user_id: i32,
    #[prost(int32, tag = "3")]
    pub withdraw_amount: i32,
    #[prost(string, tag = "4")]
    pub withdraw_time: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub created_at: ::prost::alloc::string::String,
    #[prost(string, tag = "6")]
    pub updated_at: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ApiResponseWithdrawResponse {
    #[prost(string, tag = "1")]
    pub status: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "3")]
    pub data: ::core::option::Option<WithdrawResponse>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ApiResponsesWithdrawResponse {
    #[prost(string, tag = "1")]
    pub status: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub data: ::prost::alloc::vec::Vec<WithdrawResponse>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ApiResponsesWithdrawPaginated {
    #[prost(string, tag = "1")]
    pub status: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub data: ::prost::alloc::vec::Vec<WithdrawResponse>,
    #[prost(message, optional, tag = "4")]
    pub pagination: ::core::option::Option<super::api::Pagination>,
}
/// Generated client implementations.
pub mod withdraw_service_client {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct WithdrawServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl WithdrawServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> WithdrawServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::Body>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + std::marker::Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + std::marker::Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> WithdrawServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::Body>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::Body>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::Body>,
            >>::Error: Into<StdError> + std::marker::Send + std::marker::Sync,
        {
            WithdrawServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        pub async fn find_all_withdraw(
            &mut self,
            request: impl tonic::IntoRequest<super::FindAllWithdrawRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponsesWithdrawPaginated>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/withdraw.WithdrawService/FindAllWithdraw",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("withdraw.WithdrawService", "FindAllWithdraw"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn find_withdraw_by_id(
            &mut self,
            request: impl tonic::IntoRequest<super::FindWithdrawByIdRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponseWithdrawResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/withdraw.WithdrawService/FindWithdrawById",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("withdraw.WithdrawService", "FindWithdrawById"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn find_withdraw_by_user_id(
            &mut self,
            request: impl tonic::IntoRequest<super::FindWithdrawByUserIdRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponseWithdrawResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/withdraw.WithdrawService/FindWithdrawByUserId",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("withdraw.WithdrawService", "FindWithdrawByUserId"),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn find_withdraw_by_users_id(
            &mut self,
            request: impl tonic::IntoRequest<super::FindWithdrawByUserIdRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponsesWithdrawResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/withdraw.WithdrawService/FindWithdrawByUsersId",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("withdraw.WithdrawService", "FindWithdrawByUsersId"),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn create_withdraw(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateWithdrawRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponseWithdrawResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/withdraw.WithdrawService/CreateWithdraw",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("withdraw.WithdrawService", "CreateWithdraw"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn update_withdraw(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateWithdrawRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponseWithdrawResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/withdraw.WithdrawService/UpdateWithdraw",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("withdraw.WithdrawService", "UpdateWithdraw"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn delete_withdraw(
            &mut self,
            request: impl tonic::IntoRequest<super::FindWithdrawByIdRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::api::ApiResponseEmpty>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/withdraw.WithdrawService/DeleteWithdraw",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("withdraw.WithdrawService", "DeleteWithdraw"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod withdraw_service_server {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with WithdrawServiceServer.
    #[async_trait]
    pub trait WithdrawService: std::marker::Send + std::marker::Sync + 'static {
        async fn find_all_withdraw(
            &self,
            request: tonic::Request<super::FindAllWithdrawRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponsesWithdrawPaginated>,
            tonic::Status,
        >;
        async fn find_withdraw_by_id(
            &self,
            request: tonic::Request<super::FindWithdrawByIdRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponseWithdrawResponse>,
            tonic::Status,
        >;
        async fn find_withdraw_by_user_id(
            &self,
            request: tonic::Request<super::FindWithdrawByUserIdRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponseWithdrawResponse>,
            tonic::Status,
        >;
        async fn find_withdraw_by_users_id(
            &self,
            request: tonic::Request<super::FindWithdrawByUserIdRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponsesWithdrawResponse>,
            tonic::Status,
        >;
        async fn create_withdraw(
            &self,
            request: tonic::Request<super::CreateWithdrawRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponseWithdrawResponse>,
            tonic::Status,
        >;
        async fn update_withdraw(
            &self,
            request: tonic::Request<super::UpdateWithdrawRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApiResponseWithdrawResponse>,
            tonic::Status,
        >;
        async fn delete_withdraw(
            &self,
            request: tonic::Request<super::FindWithdrawByIdRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::api::ApiResponseEmpty>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct WithdrawServiceServer<T> {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    impl<T> WithdrawServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for WithdrawServiceServer<T>
    where
        T: WithdrawService,
        B: Body + std::marker::Send + 'static,
        B::Error: Into<StdError> + std::marker::Send + 'static,
    {
        type Response = http::Response<tonic::body::Body>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            match req.uri().path() {
                "/withdraw.WithdrawService/FindAllWithdraw" => {
                    #[allow(non_camel_case_types)]
                    struct FindAllWithdrawSvc<T: WithdrawService>(pub Arc<T>);
                    impl<
                        T: WithdrawService,
                    > tonic::server::UnaryService<super::FindAllWithdrawRequest>
                    for FindAllWithdrawSvc<T> {
                        type Response = super::ApiResponsesWithdrawPaginated;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FindAllWithdrawRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WithdrawService>::find_all_withdraw(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = FindAllWithdrawSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/withdraw.WithdrawService/FindWithdrawById" => {
                    #[allow(non_camel_case_types)]
                    struct FindWithdrawByIdSvc<T: WithdrawService>(pub Arc<T>);
                    impl<
                        T: WithdrawService,
                    > tonic::server::UnaryService<super::FindWithdrawByIdRequest>
                    for FindWithdrawByIdSvc<T> {
                        type Response = super::ApiResponseWithdrawResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FindWithdrawByIdRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WithdrawService>::find_withdraw_by_id(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = FindWithdrawByIdSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/withdraw.WithdrawService/FindWithdrawByUserId" => {
                    #[allow(non_camel_case_types)]
                    struct FindWithdrawByUserIdSvc<T: WithdrawService>(pub Arc<T>);
                    impl<
                        T: WithdrawService,
                    > tonic::server::UnaryService<super::FindWithdrawByUserIdRequest>
                    for FindWithdrawByUserIdSvc<T> {
                        type Response = super::ApiResponseWithdrawResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FindWithdrawByUserIdRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WithdrawService>::find_withdraw_by_user_id(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = FindWithdrawByUserIdSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/withdraw.WithdrawService/FindWithdrawByUsersId" => {
                    #[allow(non_camel_case_types)]
                    struct FindWithdrawByUsersIdSvc<T: WithdrawService>(pub Arc<T>);
                    impl<
                        T: WithdrawService,
                    > tonic::server::UnaryService<super::FindWithdrawByUserIdRequest>
                    for FindWithdrawByUsersIdSvc<T> {
                        type Response = super::ApiResponsesWithdrawResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FindWithdrawByUserIdRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WithdrawService>::find_withdraw_by_users_id(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = FindWithdrawByUsersIdSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/withdraw.WithdrawService/CreateWithdraw" => {
                    #[allow(non_camel_case_types)]
                    struct CreateWithdrawSvc<T: WithdrawService>(pub Arc<T>);
                    impl<
                        T: WithdrawService,
                    > tonic::server::UnaryService<super::CreateWithdrawRequest>
                    for CreateWithdrawSvc<T> {
                        type Response = super::ApiResponseWithdrawResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CreateWithdrawRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WithdrawService>::create_withdraw(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = CreateWithdrawSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/withdraw.WithdrawService/UpdateWithdraw" => {
                    #[allow(non_camel_case_types)]
                    struct UpdateWithdrawSvc<T: WithdrawService>(pub Arc<T>);
                    impl<
                        T: WithdrawService,
                    > tonic::server::UnaryService<super::UpdateWithdrawRequest>
                    for UpdateWithdrawSvc<T> {
                        type Response = super::ApiResponseWithdrawResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UpdateWithdrawRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WithdrawService>::update_withdraw(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = UpdateWithdrawSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/withdraw.WithdrawService/DeleteWithdraw" => {
                    #[allow(non_camel_case_types)]
                    struct DeleteWithdrawSvc<T: WithdrawService>(pub Arc<T>);
                    impl<
                        T: WithdrawService,
                    > tonic::server::UnaryService<super::FindWithdrawByIdRequest>
                    for DeleteWithdrawSvc<T> {
                        type Response = super::super::api::ApiResponseEmpty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FindWithdrawByIdRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as WithdrawService>::delete_withdraw(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = DeleteWithdrawSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        let mut response = http::Response::new(
                            tonic::body::Body::default(),
                        );
                        let headers = response.headers_mut();
                        headers
                            .insert(
                                tonic::Status::GRPC_STATUS,
                                (tonic::Code::Unimplemented as i32).into(),
                            );
                        headers
                            .insert(
                                http::header::CONTENT_TYPE,
                                tonic::metadata::GRPC_CONTENT_TYPE,
                            );
                        Ok(response)
                    })
                }
            }
        }
    }
    impl<T> Clone for WithdrawServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    /// Generated gRPC service name
    pub const SERVICE_NAME: &str = "withdraw.WithdrawService";
    impl<T> tonic::server::NamedService for WithdrawServiceServer<T> {
        const NAME: &'static str = SERVICE_NAME;
    }
}
