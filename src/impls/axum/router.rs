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
    router::{Handler, Middleware, MiddlewareRoutes, Nested, Route, Router, Routes},
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

impl<M, R, S> IntoAxumRouter<S> for MiddlewareRoutes<M, R>
where
    M: Clone + Send + Sync + 'static,
    R: IntoAxumRouterWithMiddleware<M, S>,
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_router(self, state: S) -> AxumRouter<S> {
        let (middleware, routes) = self.into_parts();

        routes.into_axum_router_with_middleware(state, middleware)
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

/*
 * Middleware-aware router conversion.
 */

trait IntoAxumRouterWithMiddleware<M, S>
where
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_router_with_middleware(self, state: S, middleware: M) -> AxumRouter<S>;
}

impl<M, S> IntoAxumRouterWithMiddleware<M, S> for ()
where
    M: Clone + Send + Sync + 'static,
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_router_with_middleware(self, state: S, _middleware: M) -> AxumRouter<S> {
        AxumRouter::new().with_state(state)
    }
}

impl<M, T, Tail, S> IntoAxumRouterWithMiddleware<M, S> for Routes<T, Tail>
where
    M: Clone + Send + Sync + 'static,
    T: IntoAxumRouteWithMiddleware<M, S>,
    Tail: IntoAxumRouterWithMiddleware<M, S>,
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_router_with_middleware(self, state: S, middleware: M) -> AxumRouter<S> {
        let (item, tail) = self.into_parts();

        let router = tail.into_axum_router_with_middleware(state.clone(), middleware.clone());

        item.into_axum_route_with_middleware(router, state, middleware)
    }
}

impl<M, P, R, S> IntoAxumRouteWithMiddleware<M, S> for Nested<P, R>
where
    M: Clone + Send + Sync + 'static,
    P: AsRef<str>,
    R: IntoAxumRouterWithMiddleware<M, S>,
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_route_with_middleware(
        self,
        router: AxumRouter<S>,
        state: S,
        middleware: M,
    ) -> AxumRouter<S> {
        let (prefix, routes) = self.into_parts();

        let nested = routes.into_axum_router_with_middleware(state, middleware);

        router.nest(prefix.as_ref(), nested)
    }
}

trait IntoAxumRouteWithMiddleware<M, S>
where
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_route_with_middleware(
        self,
        router: AxumRouter<S>,
        state: S,
        middleware: M,
    ) -> AxumRouter<S>;
}

impl<H, M, S> IntoAxumRouteWithMiddleware<M, S> for Route<H>
where
    H: Handler<Req, Res, S> + Clone + Send + Sync + 'static,
    M: Middleware<Req, Res, S> + Clone + Send + Sync + 'static,
    M::Handler<H>: Handler<Req, Res, S> + Clone + Send + Sync + 'static,
    S: Clone + Send + Sync + 'static,
{
    fn into_axum_route_with_middleware(
        self,
        router: AxumRouter<S>,
        _state: S,
        middleware: M,
    ) -> AxumRouter<S> {
        let method = self.method().clone();
        let path = self.path().to_owned();

        /*
         * This is the important part:
         *
         *     H
         *     ↓
         * M.wrap(H)
         *     ↓
         * MiddlewareHandler<M, H>
         *     ↓
         * AxumHandler
         *
         * The middleware remains statically dispatched.
         */
        let handler = middleware.wrap(self.into_handler());

        let handler = AxumHandler {
            handler,
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

/*
 * Axum requires its Handler future to be Send + 'static.
 *
 * The g-server handler/middleware chain itself remains
 * statically dispatched. The Axum adapter is the framework
 * boundary.
 */
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
    pub fn into_axum(self) -> AxumRouter<()> {
        let (routes, state) = self.into_parts();

        routes
            .into_axum_router(state.clone())
            .with_state::<()>(state)
    }
}
