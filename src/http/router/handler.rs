use std::future::Future;

pub trait Handler<Req, Res, S = ()> {
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
