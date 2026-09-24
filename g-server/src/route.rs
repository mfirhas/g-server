use crate::{Request, Response};

pub type Handler<C, P, Q, ReqB, Fut> = fn(cx: C, req: Request<P, Q, ReqB>) -> Fut;

pub type Middleware<F, C, P, Q, ReqB, Fut> =
    fn(cx: C, req: Request<P, Q, ReqB>, ex: Executor<F>) -> Fut;

/// Http methods supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Options,
    Head,
    Trace,
    Query,
    Connect,
    Any,
}

impl From<crate::http::Method> for HttpMethod {
    #[inline]
    fn from(value: crate::http::Method) -> Self {
        match value {
            crate::http::Method::HEAD => Self::Head,
            crate::http::Method::GET => Self::Get,
            crate::http::Method::POST => Self::Post,
            crate::http::Method::PUT => Self::Put,
            crate::http::Method::PATCH => Self::Patch,
            crate::http::Method::DELETE => Self::Delete,
            crate::http::Method::OPTIONS => Self::Options,
            crate::http::Method::TRACE => Self::Trace,
            crate::http::Method::QUERY => Self::Query,
            crate::http::Method::CONNECT => Self::Connect,
            _ => Self::Any,
        }
    }
}

/// Contains all middlewares(if any) and handler.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Executor<F> {
    func: F,
}

impl<F> Executor<F> {
    #[inline]
    pub fn new<C, P, Q, ReqB, ResB, ErrB, Fut>(func: F) -> Self
    where
        F: FnOnce(C, Request<P, Q, ReqB>) -> Fut,
        C: Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<Response<ResB>, Response<ErrB>>> + Send,
    {
        Self { func }
    }

    #[inline]
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

pub async fn unimplemented_handler<C, P, Q, ReqB, ResB>(
    _: C,
    _: Request<P, Q, ReqB>,
) -> Result<Response<ResB>, Response<String>> {
    Err(Response::new()
        .with_status(crate::http::StatusCode::NOT_IMPLEMENTED)
        .with_text("g-server: not implemented".into()))
}

pub async fn file_handler<C, P, Q, ReqB>(
    _: C,
    _: Request<P, Q, ReqB>,
) -> Result<Response<()>, Response<String>> {
    Ok(Response::from(crate::StatusCode::OK))
}
