use std::future::Future;

use super::Handler;

pub trait Middleware<Req, Res, S = ()>
where
    Req: Send,
    S: Sync,
    Self: Sync,
{
    type Handler<H>: Handler<Req, Res, S>
    where
        H: Handler<Req, Res, S> + Sync;

    fn wrap<H>(self, handler: H) -> Self::Handler<H>
    where
        H: Handler<Req, Res, S> + Sync;
}

pub trait MiddlewareFn<Req, Res, S = ()>: Sync
where
    Req: Send,
    S: Sync,
{
    type Future<'a, H>: Future<Output = Res> + Send + 'a
    where
        Self: 'a,
        S: 'a,
        H: Handler<Req, Res, S> + Sync + 'a;

    fn call<'a, H>(&'a self, state: &'a S, request: Req, next: &'a H) -> Self::Future<'a, H>
    where
        H: Handler<Req, Res, S> + Sync + 'a;
}

pub struct MiddlewareHandler<M, H> {
    middleware: M,
    handler: H,
}

impl<M, H> MiddlewareHandler<M, H> {
    pub fn new(middleware: M, handler: H) -> Self {
        Self {
            middleware,
            handler,
        }
    }

    pub fn middleware(&self) -> &M {
        &self.middleware
    }

    pub fn handler(&self) -> &H {
        &self.handler
    }

    pub fn into_parts(self) -> (M, H) {
        (self.middleware, self.handler)
    }
}

impl<Req, Res, S, M> Middleware<Req, Res, S> for M
where
    Req: Send,
    S: Sync,
    M: MiddlewareFn<Req, Res, S> + Sync,
{
    type Handler<H>
        = MiddlewareHandler<M, H>
    where
        H: Handler<Req, Res, S> + Sync;

    fn wrap<H>(self, handler: H) -> Self::Handler<H>
    where
        H: Handler<Req, Res, S> + Sync,
    {
        MiddlewareHandler::new(self, handler)
    }
}

impl<Req, Res, S, M, H> Handler<Req, Res, S> for MiddlewareHandler<M, H>
where
    Req: Send,
    S: Sync,
    M: MiddlewareFn<Req, Res, S> + Sync,
    H: Handler<Req, Res, S> + Sync,
{
    type Future<'a>
        = M::Future<'a, H>
    where
        Self: 'a,
        S: 'a,
        M: 'a,
        H: 'a;

    fn call<'a>(&'a self, state: &'a S, request: Req) -> Self::Future<'a> {
        self.middleware.call(state, request, &self.handler)
    }
}

pub struct MiddlewareRoutes<M, R> {
    middleware: M,
    routes: R,
}

impl<M, R> MiddlewareRoutes<M, R> {
    pub(crate) fn new(middleware: M, routes: R) -> Self {
        Self { middleware, routes }
    }

    pub(crate) fn into_parts(self) -> (M, R) {
        (self.middleware, self.routes)
    }
}
