use std::marker::PhantomData;

use crate::{Config, Request, Response};

pub type Handler<C, P, Q, ReqB, Fut> = fn(cx: C, req: Request<P, Q, ReqB>) -> Fut;

pub type Middleware<F, C, P, Q, ReqB, ResB> =
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
        Fut: Future<Output = Response<ResB>> + Send,
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
        Fut: Future<Output = Response<ResB>> + Send,
    {
        (self.func)(cx, req).await
    }
}

/// Route represents 1 endpoint execution.
pub struct Route<F, P = (), Q = (), B = ()> {
    pub method: HttpMethod,
    pub endpoint: &'static str,
    pub config: Config,
    pub response_body_type: ResponseBodyType,
    pub executor: Executor<F>,

    pub _path_params: PhantomData<P>,
    pub _query_params: PhantomData<Q>,
    pub _request_body: PhantomData<B>,
}

impl<F, P, Q, ReqB> Route<F, P, Q, ReqB> {
    pub fn new<C, M, ResB, Fut>(
        method: HttpMethod,
        endpoint: &'static str,
        config: Config,
        response_body_type: ResponseBodyType,
        executor: Executor<F>,
    ) -> Self
    where
        C: Clone + Send + Sync + 'static,
        F: FnOnce(C, Request<P, Q, ReqB>) -> Fut,
        Fut: Future<Output = Response<ResB>> + Send,
    {
        Self {
            method,
            endpoint,
            config,
            response_body_type,
            executor,
            _path_params: PhantomData,
            _query_params: PhantomData,
            _request_body: PhantomData,
        }
    }
}
