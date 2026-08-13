use std::borrow::Cow;
use std::marker::PhantomData;

use crate::http::Method;

use super::{Handler, Route};

pub struct Router<Req, Res, R = (), S = ()> {
    routes: R,
    state: S,
    _marker: PhantomData<fn(Req) -> Res>,
}

pub struct Routes<T, Tail> {
    item: T,
    tail: Tail,
}

impl<T, Tail> Routes<T, Tail> {
    pub fn item(&self) -> &T {
        &self.item
    }

    pub fn tail(&self) -> &Tail {
        &self.tail
    }

    pub fn into_parts(self) -> (T, Tail) {
        (self.item, self.tail)
    }
}

pub struct Nested<P, R> {
    prefix: P,
    routes: R,
}

impl<P, R> Nested<P, R> {
    pub fn prefix(&self) -> &P {
        &self.prefix
    }

    pub fn routes(&self) -> &R {
        &self.routes
    }

    pub fn into_parts(self) -> (P, R) {
        (self.prefix, self.routes)
    }
}

impl<Req, Res> Router<Req, Res> {
    pub fn new() -> Self {
        Self {
            routes: (),
            state: (),
            _marker: PhantomData,
        }
    }
}

impl<Req, Res> Default for Router<Req, Res> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Req, Res, R, S> Router<Req, Res, R, S> {
    pub fn with_state<NS>(self, state: NS) -> Router<Req, Res, R, NS> {
        Router {
            routes: self.routes,
            state,
            _marker: PhantomData,
        }
    }

    pub fn route<H>(self, route: Route<H>) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        Router {
            routes: Routes {
                item: route,
                tail: self.routes,
            },
            state: self.state,
            _marker: PhantomData,
        }
    }

    pub fn nest<P, NR>(
        self,
        prefix: P,
        router: Router<Req, Res, NR, S>,
    ) -> Router<Req, Res, Routes<Nested<Cow<'static, str>, NR>, R>, S>
    where
        P: Into<Cow<'static, str>>,
    {
        Router {
            routes: Routes {
                item: Nested {
                    prefix: prefix.into(),
                    routes: router.routes,
                },
                tail: self.routes,
            },
            state: self.state,
            _marker: PhantomData,
        }
    }

    pub fn into_routes(self) -> R {
        self.routes
    }

    pub fn into_parts(self) -> (R, S) {
        (self.routes, self.state)
    }

    pub fn get<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Get, path, handler))
    }

    pub fn post<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Post, path, handler))
    }

    pub fn put<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Put, path, handler))
    }

    pub fn patch<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Patch, path, handler))
    }

    pub fn delete<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Delete, path, handler))
    }

    pub fn head<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Head, path, handler))
    }

    pub fn options<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Options, path, handler))
    }

    pub fn connect<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Connect, path, handler))
    }

    pub fn trace<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Trace, path, handler))
    }

    pub fn query<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<Route<H>, R>, S>
    where
        H: Handler<Req, Res, S>,
    {
        self.route(Route::new(Method::Query, path, handler))
    }
}
