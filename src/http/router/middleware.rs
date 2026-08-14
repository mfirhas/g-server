use std::future::Future;

use super::Handler;

/// Transforms a handler into another handler.
///
/// Middleware can:
///
/// - run before the handler
/// - modify the request
/// - short-circuit
/// - call the next handler
/// - modify the response
/// - run after the handler
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

/// Defines the actual behavior of middleware.
///
/// `next` is the concrete handler wrapped by this middleware.
///
/// The middleware may call `next.call(...)`, or return a response
/// directly to short-circuit the chain.
pub trait MiddlewareFn<Req, Res, S = ()>: Sync
where
    Req: Send,
    S: Sync,
{
    fn call<H>(&self, state: &S, request: Req, next: &H) -> impl Future<Output = Res> + Send
    where
        H: Handler<Req, Res, S> + Sync;
}

/// Concrete handler produced by middleware.
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
    fn call(&self, state: &S, request: Req) -> impl Future<Output = Res> + Send {
        async { self.middleware.call(state, request, &self.handler).await }
    }
}
