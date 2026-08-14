use std::{future::Future, marker::PhantomData, pin::Pin};

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

pub trait IntoAxumRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_router(self, state: S) -> AxumRouter<S>;
}

impl<S> IntoAxumRouter<S> for ()
where
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_router(self, state: S) -> AxumRouter<S> {
        AxumRouter::new().with_state(state)
    }
}

impl<T, Tail, S> IntoAxumRouter<S> for Routes<T, Tail>
where
    T: IntoAxumRoute<S>,
    Tail: IntoAxumRouter<S>,
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_router(self, state: S) -> AxumRouter<S> {
        let (item, tail) = self.into_parts();

        let router = tail.into_axum_router(state.clone());

        item.into_axum_route(router, state)
    }
}

trait IntoAxumRoute<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_route(self, router: AxumRouter<S>, state: S) -> AxumRouter<S>;
}

impl<H, S> IntoAxumRoute<S> for Route<H>
where
    H: Handler<Req, Res, S> + Clone + Send + Sync + 'static,
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_route(self, router: AxumRouter<S>, _state: S) -> AxumRouter<S> {
        let method = self.method().clone();
        let path = self.path().to_owned();

        let handler = AxumHandler {
            handler: self.into_handler(),
            _state: PhantomData,
        };

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

impl<P, R, S> IntoAxumRoute<S> for Nested<P, R>
where
    P: AsRef<str>,
    R: IntoAxumRouter<S>,
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_route(self, router: AxumRouter<S>, state: S) -> AxumRouter<S> {
        let (prefix, routes) = self.into_parts();

        router.nest(prefix.as_ref(), routes.into_axum_router(state))
    }
}

struct AxumHandler<H, S> {
    handler: H,
    _state: PhantomData<fn() -> S>,
}

impl<H, S> Clone for AxumHandler<H, S>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            _state: PhantomData,
        }
    }
}

impl<H, S> axum::handler::Handler<(), S> for AxumHandler<H, S>
where
    H: Handler<Req, Res, S> + Clone + Send + Sync + 'static,
    S: Clone + Send + Sync + 'static,
{
    type Future = Pin<Box<dyn Future<Output = AxumResponse> + Send + 'static>>;

    fn call(self, request: AxumRequest<Body>, state: S) -> Self::Future {
        Box::pin(async move {
            let request = Req::from(request);

            let response = self.handler.call(&state, request).await;

            AxumResponse::from(response)
        })
    }
}

impl<R, S> Router<Req, Res, R, S>
where
    R: IntoAxumRouter<S>,
    S: Clone + Send + Sync + 'static,
{
    pub fn into_axum(self) -> AxumRouter<S> {
        let (routes, state) = self.into_parts();

        routes.into_axum_router(state)
    }
}
