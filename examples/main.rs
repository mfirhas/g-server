use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use g_server::{
    Config, IntoAxumHtmlResponse, IntoAxumJsonResponse, IntoAxumStringResponse, Request, Response,
    route::{Executor, HttpMethod, ResponseBodyType, Route},
};
use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct PathParams {
    pub user_id: u64,
    pub user_email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueryParams {
    pub user_name: String,
    pub user_contact: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestBody {
    pub action: u32,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct Context {
    data: u32,
}

impl Context {
    pub fn init() -> Self {
        Self { data: 123 }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody {
    pub user_id: u64,
    pub user_name: String,
    pub token: String,
    pub data: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequestBody_2 {
    pub user_id: u64,
    pub action: u32,
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody_2 {
    pub action: u32,
    pub token: String,
    pub data: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    msg: String,
}

pub async fn handler_1(
    cx: Context,
    req: Request<PathParams, QueryParams, RequestBody>,
) -> Result<Response<ResponseBody>, Response<ErrorResponse>> {
    Ok(Response {
        status: StatusCode::OK,
        headers: HeaderMap::new(),
        body: ResponseBody {
            user_id: req.path_params.user_id,
            user_name: req.query_params.user_name,
            token: req.body.token,
            data: cx.data,
        },
    })
}

pub async fn handler_2(
    cx: Context,
    req: Request<(), (), RequestBody_2>,
) -> Result<Response<ResponseBody_2>, Response<ErrorResponse>> {
    Ok(Response {
        status: StatusCode::OK,
        headers: HeaderMap::new(),
        body: ResponseBody_2 {
            action: req.body.action,
            token: req.body.token,
            data: cx.data,
        },
    })
}

pub async fn auth<P, Q, ReqB, ResB, F, Fut>(
    cx: Context,
    req: Request<P, Q, ReqB>,
    next: Executor<F>,
) -> Result<Response<ResB>, Response<ErrorResponse>>
where
    F: FnOnce(Context, Request<P, Q, ReqB>) -> Fut,
    Fut: Future<Output = Result<Response<ResB>, Response<ErrorResponse>>> + Send,
{
    println!("before auth...");

    let resp = next.exec(cx, req).await?;

    println!("after auth...");

    Ok(resp)
}

pub async fn logger<P, Q, ReqB, ResB, F, Fut>(
    cx: Context,
    req: Request<P, Q, ReqB>,
    next: Executor<F>,
) -> Result<Response<ResB>, Response<ErrorResponse>>
where
    F: FnOnce(Context, Request<P, Q, ReqB>) -> Fut,
    Fut: Future<Output = Result<Response<ResB>, Response<ErrorResponse>>> + Send,
{
    println!("before log...");

    let resp = next.exec(cx, req).await?;

    println!("after log...");

    Ok(resp)
}

#[tokio::main]
async fn main() {
    let server_name = "app_a";
    let ip_address = "0.0.0.0";
    let port = 42069_u16;

    let global_config = Config::default();
    // if user supply any config, we define them here:
    // global_config.timeout = Some(X); // X = user defined timeout from macro
    // ...

    let context = Context::init(); // Context is from macro, appended with `::init()`

    // list of routes
    let router = init_app_a(global_config.clone());
    let router = __route_handler_1(router, global_config.clone());
    let router = __route_handler_2(router, global_config.clone());

    let router = router.with_state(context);

    let app_a_listener = ::tokio::net::TcpListener::bind((ip_address, port))
        .await
        .expect("failed creating app_a tcp listener");

    ::tokio::try_join!(::axum::serve(app_a_listener, router),)
        .expect("failed running the server...");
}

pub fn init_app_a(global_config: Config) -> Router<Context> {
    let mut router = Router::new();

    // if let Some(timeout) = global_config.timeout {
    //     router = router.layer(TimeoutLayer::new(Duration::from_millis(timeout.into())));
    // }

    router
}

pub fn __route_handler_1(
    router: axum::Router<Context>,
    global_config: Config,
) -> axum::Router<Context> {
    let config = global_config;
    // if user supply any config, we define them here:
    // global_config.timeout = // user defined timeout from macro
    // ...
    let executor = Executor::new(handler_1);
    // We assemble middlewares from last to first: first in array execute first, so we declare last here to make it executed first.
    // We assemble these middlewares directly from macro declaration.
    // If no middlewares, straight to route.
    let executor = Executor::new(move |cx, req| logger(cx, req, executor));
    let executor = Executor::new(move |cx, req| auth(cx, req, executor));
    let route = Route::<_> {
        method: HttpMethod::Post,
        endpoint: "/route_1/{user_id}/{user_email}",
        config,
        response_body_type: ResponseBodyType::Json,
        executor: executor,
    };

    #[rustfmt::skip]
    let route_handler = move |
          State(cx): State<Context>,
          headers: axum::http::HeaderMap,
          Path(path_params): Path<PathParams>,
          Query(query_params): Query<QueryParams>,
          Json(body): Json<RequestBody>,
    | async move {
        let req = Request {
            headers,
            path_params,
            query_params,
            body,
        };

        route.executor.exec(cx, req).await.into_json_response()
    };

    let router = match route.method {
        HttpMethod::Get => router.route(route.endpoint, axum::routing::get(route_handler)),

        HttpMethod::Post => router.route(route.endpoint, axum::routing::post(route_handler)),

        HttpMethod::Put => router.route(route.endpoint, axum::routing::put(route_handler)),

        HttpMethod::Patch => router.route(route.endpoint, axum::routing::patch(route_handler)),

        HttpMethod::Head => router.route(route.endpoint, axum::routing::head(route_handler)),

        HttpMethod::Query => router.route(route.endpoint, axum::routing::get(route_handler)),

        HttpMethod::Any => router.route(route.endpoint, axum::routing::any(route_handler)),
    };

    router
}

pub fn __route_handler_2(
    router: axum::Router<Context>,
    global_config: Config,
) -> axum::Router<Context> {
    let config = global_config;
    let executor = Executor::new(handler_2);
    // We assemble middlewares from last to first: first in array execute first, so we declare last here to make it executed first.
    // We assemble these middlewares directly from macro declaration.
    // If no middlewares, straight to route.
    let executor = Executor::new(move |cx, req| logger(cx, req, executor));
    let executor = Executor::new(move |cx, req| auth(cx, req, executor));
    let route = Route::<_> {
        method: HttpMethod::Post,
        endpoint: "/route_2",
        config,
        response_body_type: ResponseBodyType::Json,
        executor: executor,
    };

    #[rustfmt::skip]
    let route_handler = move |
          State(cx): State<Context>,
          headers: axum::http::HeaderMap,
          Path(path_params): Path<()>,
          Query(query_params): Query<()>,
          Json(body): Json<RequestBody_2>,
    | async move {
        let req = Request {
            headers,
            path_params,
            query_params,
            body,
        };

        route.executor.exec(cx, req).await.into_json_response()
    };

    let router = match route.method {
        HttpMethod::Get => router.route(route.endpoint, axum::routing::get(route_handler)),

        HttpMethod::Post => router.route(route.endpoint, axum::routing::post(route_handler)),

        HttpMethod::Put => router.route(route.endpoint, axum::routing::put(route_handler)),

        HttpMethod::Patch => router.route(route.endpoint, axum::routing::patch(route_handler)),

        HttpMethod::Head => router.route(route.endpoint, axum::routing::head(route_handler)),

        HttpMethod::Query => router.route(route.endpoint, axum::routing::get(route_handler)),

        HttpMethod::Any => router.route(route.endpoint, axum::routing::any(route_handler)),
    };

    router
}
