use crate::{Config, Request, Response};

pub type Handler<C, P, Q, ReqB, ResB> = fn(cx: C, req: Request<P, Q, ReqB>) -> Response<ResB>;

pub type Middleware<C, P, Q, ReqB, ResB, F> =
    fn(cx: C, req: Request<P, Q, ReqB>, ex: Executor<F>) -> Response<ResB>;

/// Http methods supported.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Head,
    Get,
    Put,
    Post,
    Patch,
    Query,
    Any,
}

/// Response body supported.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseBodyType {
    #[default]
    Json,
    String,
    Html,
}

/// Contains all middlewares(if any) and handler.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Executor<F> {
    func: F,
}

impl<F> Executor<F> {
    pub fn new<C, P, Q, ReqB, ResB, Fut>(func: F) -> Self
    where
        F: FnOnce(C, Request<P, Q, ReqB>) -> Fut,
        C: Clone + Send + Sync + 'static,
        Fut: Future<Output = Response<ResB>>,
    {
        Self { func }
    }

    pub async fn exec<C, P, Q, ReqB, ResB, Fut>(
        self,
        cx: C,
        req: Request<P, Q, ReqB>,
    ) -> Response<ResB>
    where
        F: FnOnce(C, Request<P, Q, ReqB>) -> Fut,
        C: Clone + Send + Sync + 'static,
        Fut: Future<Output = Response<ResB>>,
    {
        (self.func)(cx, req).await
    }
}

pub struct Route<F, P = (), Q = (), B = ()> {
    pub method: HttpMethod,
    pub endpoint: &'static str,
    pub config: Config,
    pub path_params: P,
    pub query_params: Q,
    pub request_body: B,
    pub response_body_type: ResponseBodyType,
    pub executor: Executor<F>,
}
