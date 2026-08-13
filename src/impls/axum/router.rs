use std::{future::Future, pin::Pin};

use axum::{
    Router as AxumRouter,
    body::Body,
    http::Request as AxumRequest,
    response::Response as AxumResponse,
    routing::{any, delete, get, head, options, patch, post, put, trace},
};

use crate::http::{
    Request, Response,
    router::{Handler, Nested, Route, Router, Routes},
};

type Req = Request<axum::http::Uri, axum::http::HeaderMap, Body>;
type Res = Response<axum::http::HeaderMap, Body>;

pub trait IntoAxumRouter {
    fn into_axum_router(self) -> AxumRouter;
}

impl IntoAxumRouter for () {
    fn into_axum_router(self) -> AxumRouter {
        AxumRouter::new()
    }
}

impl<T, Tail> IntoAxumRouter for Routes<T, Tail>
where
    T: IntoAxumRoute,
    Tail: IntoAxumRouter,
{
    fn into_axum_router(self) -> AxumRouter {
        let (item, tail) = self.into_parts();

        item.into_axum_route(tail.into_axum_router())
    }
}

trait IntoAxumRoute {
    fn into_axum_route(self, router: AxumRouter) -> AxumRouter;
}

impl<H> IntoAxumRoute for Route<H>
where
    H: Handler<Req, Res> + Clone + Send + Sync + 'static,
    H::Future: Send + 'static,
{
    fn into_axum_route(self, router: AxumRouter) -> AxumRouter {
        let method = self.method().clone();
        let path = self.path().to_owned();
        let handler = AxumHandler(self.into_handler());

        match method {
            crate::http::Method::Get => router.route(&path, get(handler)),

            crate::http::Method::Post => router.route(&path, post(handler)),

            crate::http::Method::Put => router.route(&path, put(handler)),

            crate::http::Method::Patch => router.route(&path, patch(handler)),

            crate::http::Method::Delete => router.route(&path, delete(handler)),

            crate::http::Method::Head => router.route(&path, head(handler)),

            crate::http::Method::Options => router.route(&path, options(handler)),

            crate::http::Method::Trace => router.route(&path, trace(handler)),

            crate::http::Method::Connect
            | crate::http::Method::Query
            | crate::http::Method::Custom(_) => router.route(&path, any(handler)),
        }
    }
}

impl<P, R> IntoAxumRoute for Nested<P, R>
where
    P: AsRef<str>,
    R: IntoAxumRouter,
{
    fn into_axum_route(self, router: AxumRouter) -> AxumRouter {
        let (prefix, routes) = self.into_parts();

        router.nest(prefix.as_ref(), routes.into_axum_router())
    }
}

struct AxumHandler<H>(H);

impl<H> Clone for AxumHandler<H>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<H> axum::handler::Handler<(), ()> for AxumHandler<H>
where
    H: Handler<Req, Res> + Clone + Send + Sync + 'static,
    H::Future: Send + 'static,
{
    type Future = Pin<Box<dyn Future<Output = AxumResponse> + Send + 'static>>;

    fn call(self, request: AxumRequest<Body>, _state: ()) -> Self::Future {
        Box::pin(async move {
            let request = Req::from(request);
            let response = self.0.call(request).await;

            AxumResponse::from(response)
        })
    }
}

impl<R> Router<Req, Res, R>
where
    R: IntoAxumRouter,
{
    pub fn into_axum(self) -> AxumRouter {
        self.into_routes().into_axum_router()
    }
}
