use std::future::Future;

pub trait Handler<Req, Res, S = ()> {
    type Future: Future<Output = Res>;

    fn call(&self, state: &S, request: Req) -> Self::Future;
}

impl<F, Fut, Req, Res, S> Handler<Req, Res, S> for F
where
    F: Fn(&S, Req) -> Fut,
    Fut: Future<Output = Res>,
{
    type Future = Fut;

    fn call(&self, state: &S, request: Req) -> Self::Future {
        self(state, request)
    }
}
