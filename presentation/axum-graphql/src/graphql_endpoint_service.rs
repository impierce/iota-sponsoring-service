use async_graphql::Executor;
use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::{
    BoxError,
    body::{Body, HttpBody},
    http::{Request as HttpRequest, Response as HttpResponse},
};
use bytes::Bytes;
use futures_util::future::BoxFuture;
use std::{
    convert::Infallible,
    task::{Context, Poll},
};
use tower_service::Service;

/// A GraphQL endpoint service that routes between HTTP and WebSocket requests.
#[derive(Clone)]
pub struct GraphQLEndpointService<E> {
    executor: E,
}

impl<E> GraphQLEndpointService<E> {
    /// Create a GraphQLEndpointService handler.
    pub fn new(executor: E) -> Self {
        Self { executor }
    }
}

/// Implement the Service trait for GraphQLEndpointService to handle both HTTP and WebSocket requests.
impl<B, E> Service<HttpRequest<B>> for GraphQLEndpointService<E>
where
    B: HttpBody<Data = Bytes> + Send + 'static,
    B::Data: Into<Bytes>,
    B::Error: Into<BoxError>,
    E: Executor,
{
    type Response = HttpResponse<Body>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: HttpRequest<B>) -> Self::Future {
        let is_websocket = req
            .headers()
            .get("upgrade")
            .map(|header_value| header_value == "websocket")
            .unwrap_or(false);

        if is_websocket {
            let mut service = GraphQLSubscription::new(self.executor.clone());
            service.call(req)
        } else {
            let mut service = GraphQL::new(self.executor.clone());
            service.call(req)
        }
    }
}
