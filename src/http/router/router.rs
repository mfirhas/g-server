use std::borrow::Cow;
use std::marker::PhantomData;

use super::{Handler, Route};
use crate::http::Method;

pub struct Router<Req, Res, Routes = ()> {
    routes: Routes,
    _marker: PhantomData<fn(Req) -> Res>,
}

pub struct Routes<H, Tail> {
    route: Route<H>,
    tail: Tail,
}

impl<H, Tail> Routes<H, Tail> {
    pub fn route(&self) -> &Route<H> {
        &self.route
    }

    pub fn tail(&self) -> &Tail {
        &self.tail
    }

    pub fn into_parts(self) -> (Route<H>, Tail) {
        (self.route, self.tail)
    }
}

impl<Req, Res> Router<Req, Res> {
    pub fn new() -> Self {
        Self {
            routes: (),
            _marker: PhantomData,
        }
    }
}

impl<Req, Res> Default for Router<Req, Res> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Req, Res, R> Router<Req, Res, R> {
    pub fn route<H>(self, route: Route<H>) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        Router {
            routes: Routes {
                route,
                tail: self.routes,
            },
            _marker: PhantomData,
        }
    }

    pub fn get<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Get, path, handler))
    }

    pub fn post<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Post, path, handler))
    }

    pub fn put<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Put, path, handler))
    }

    pub fn patch<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Patch, path, handler))
    }

    pub fn delete<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Delete, path, handler))
    }

    pub fn head<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Head, path, handler))
    }

    pub fn options<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Options, path, handler))
    }

    pub fn connect<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Connect, path, handler))
    }

    pub fn trace<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Trace, path, handler))
    }

    pub fn query<H>(
        self,
        path: impl Into<Cow<'static, str>>,
        handler: H,
    ) -> Router<Req, Res, Routes<H, R>>
    where
        H: Handler<Req, Res>,
    {
        self.route(Route::new(Method::Query, path, handler))
    }
}
