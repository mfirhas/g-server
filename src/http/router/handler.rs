use std::future::Future;

pub trait Handler<Req, Res, S = ()> {
    fn call(&self, state: &S, request: Req) -> impl Future<Output = Res> + Send;
}

impl<F, Fut, Req, Res, S> Handler<Req, Res, S> for F
where
    F: Fn(&S, Req) -> Fut,
    Fut: Future<Output = Res> + Send,
{
    fn call(&self, state: &S, request: Req) -> impl Future<Output = Res> + Send {
        self(state, request)
    }
}
