use crate::{Config, Request, Response};

pub type Handler<C, P, Q, ReqB, Fut> = fn(cx: C, req: Request<P, Q, ReqB>) -> Fut;

pub type Middleware<F, C, P, Q, ReqB, Fut> =
    fn(cx: C, req: Request<P, Q, ReqB>, ex: Executor<F>) -> Fut;

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
    pub fn new<C, P, Q, ReqB, ResB, ErrB, Fut>(func: F) -> Self
    where
        F: FnOnce(C, Request<P, Q, ReqB>) -> Fut,
        C: Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<Response<ResB>, Response<ErrB>>> + Send,
    {
        Self { func }
    }

    pub async fn exec<C, P, Q, ReqB, ResB, ErrB, Fut>(
        self,
        cx: C,
        req: Request<P, Q, ReqB>,
    ) -> Result<Response<ResB>, Response<ErrB>>
    where
        F: FnOnce(C, Request<P, Q, ReqB>) -> Fut,
        C: Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<Response<ResB>, Response<ErrB>>> + Send,
    {
        (self.func)(cx, req).await
    }
}

/// Route represents 1 endpoint execution.
pub struct Route<F> {
    pub method: HttpMethod,
    pub endpoint: &'static str,
    pub config: Config,
    pub response_body_type: ResponseBodyType,
    pub executor: Executor<F>,
}
