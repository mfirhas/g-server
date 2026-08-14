use std::future::Future;

use crate::http::IntoResponse;

pub trait Handler<Req, Res, S = ()>
where
    Res: IntoResponse,
{
    type Future<'a>: Future<Output = Res> + Send + 'a
    where
        Self: 'a,
        S: 'a;

    fn call<'a>(&'a self, state: &'a S, request: Req) -> Self::Future<'a>;
}

impl<F, Fut, Req, Res, S> Handler<Req, Res, S> for F
where
    F: Fn(&S, Req) -> Fut,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse,
{
    type Future<'a>
        = Fut
    where
        Self: 'a,
        S: 'a;

    fn call<'a>(&'a self, state: &'a S, request: Req) -> Self::Future<'a> {
        self(state, request)
    }
}
