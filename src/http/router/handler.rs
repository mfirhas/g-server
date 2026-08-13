use std::future::Future;

pub trait Handler<Req, Res> {
    type Future: Future<Output = Res>;

    fn call(&self, request: Req) -> Self::Future;
}

impl<F, Fut, Req, Res> Handler<Req, Res> for F
where
    F: Fn(Req) -> Fut,
    Fut: Future<Output = Res>,
{
    type Future = Fut;

    fn call(&self, request: Req) -> Self::Future {
        self(request)
    }
}
