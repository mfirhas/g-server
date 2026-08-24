use serde::{Deserialize, Serialize};

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
    req: g_server::Request<PathParams, QueryParams, RequestBody>,
) -> Result<g_server::Response<ResponseBody>, g_server::Response<ErrorResponse>> {
    Ok(g_server::Response {
        status: g_server::http::StatusCode::OK,
        headers: g_server::http::HeaderMap::new(),
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
    req: g_server::Request<(), (), RequestBody_2>,
) -> Result<g_server::Response<ResponseBody_2>, g_server::Response<ErrorResponse>> {
    Ok(g_server::Response {
        status: g_server::http::StatusCode::OK,
        headers: g_server::http::HeaderMap::new(),
        body: ResponseBody_2 {
            action: req.body.action,
            token: req.body.token,
            data: cx.data,
        },
    })
}

pub async fn auth<P, Q, ReqB, ResB, F, Fut>(
    cx: Context,
    req: g_server::Request<P, Q, ReqB>,
    next: g_server::route::Executor<F>,
) -> Result<g_server::Response<ResB>, g_server::Response<ErrorResponse>>
where
    F: FnOnce(Context, g_server::Request<P, Q, ReqB>) -> Fut,
    Fut:
        Future<Output = Result<g_server::Response<ResB>, g_server::Response<ErrorResponse>>> + Send,
{
    println!("before auth...");

    let resp = next.exec(cx, req).await?;

    println!("after auth...");

    Ok(resp)
}

pub async fn logger<P, Q, ReqB, ResB, F, Fut>(
    cx: Context,
    req: g_server::Request<P, Q, ReqB>,
    next: g_server::route::Executor<F>,
) -> Result<g_server::Response<ResB>, g_server::Response<ErrorResponse>>
where
    F: FnOnce(Context, g_server::Request<P, Q, ReqB>) -> Fut,
    Fut:
        Future<Output = Result<g_server::Response<ResB>, g_server::Response<ErrorResponse>>> + Send,
{
    println!("before log...");

    let resp = next.exec(cx, req).await?;

    println!("after log...");

    Ok(resp)
}

// below this is macro generated for axum implementation
// -----------------------------------------------------

use g_server::{IntoAxumHtmlResponse, IntoAxumJsonResponse, IntoAxumStringResponse};

#[tokio::main]
async fn main() {
    let app_a = __init_app_a();

    let app_a_listener = ::tokio::net::TcpListener::bind((app_a.0.ip_address, app_a.0.port))
        .await
        .expect(format!("failed creating {} tcp listener", app_a.0.name).as_str());

    println!(
        "g-server: running {} on {}:{}...",
        app_a.0.name, app_a.0.ip_address, app_a.0.port
    );

    ::tokio::try_join!(::axum::serve(app_a_listener, app_a.1),)
        .expect("failed running the all servers...");
}

pub fn __init_app_a() -> (g_server::Server, axum::Router<()>) {
    let server = g_server::Server {
        name: "app_a",
        ip_address: "0.0.0.0",
        port: 42069,
    };

    let global_config = g_server::Config::default();
    // if user supply any config, we define them here:
    // global_config.timeout = Some(X); // X = user defined timeout from macro
    // ...

    let context = Context::init(); // Context is from macro, appended with `::init()`

    let router = axum::Router::new();

    // middlewares
    // if let Some(timeout) = global_config.timeout {
    //     router = router.layer(TimeoutLayer::new(Duration::from_millis(timeout.into())));
    // }

    // we register all routes here
    let router = __route_app_a_handler_1(router, global_config.clone());
    let router = __route_app_a_handler_2(router, global_config.clone());

    let router = router.with_state(context);

    (server, router)
}

pub fn __route_app_a_handler_1(
    router: axum::Router<Context>,
    global_config: g_server::Config,
) -> axum::Router<Context> {
    let config = global_config;
    // if user supply any config, we define them here:
    // global_config.timeout = // user defined timeout from macro
    // ...
    let executor = g_server::route::Executor::new(handler_1);
    // We assemble middlewares from last to first: first in array execute first, so we declare last here to make it executed first.
    // We assemble these middlewares directly from macro declaration.
    // If no middlewares, straight to route.
    let executor = g_server::route::Executor::new(move |cx, req| logger(cx, req, executor));
    let executor = g_server::route::Executor::new(move |cx, req| auth(cx, req, executor));
    let route = g_server::route::Route::<_> {
        method: g_server::route::HttpMethod::Post,
        endpoint: "/route_1/{user_id}/{user_email}",
        config,
        response_body_type: g_server::route::ResponseBodyType::Json,
        executor: executor,
    };

    #[rustfmt::skip]
    let route_handler = move |
          axum::extract::State(cx): axum::extract::State<Context>,
          headers: axum::http::HeaderMap,
          axum::extract::Path(path_params): axum::extract::Path<PathParams>,
          axum::extract::Query(query_params): axum::extract::Query<QueryParams>,
          axum::extract::Json(body): axum::extract::Json<RequestBody>,
    | async move {
        let req = g_server::Request {
            headers,
            path_params,
            query_params,
            body,
        };

        route.executor.exec(cx, req).await.into_json_response()
    };

    let router = match route.method {
        g_server::route::HttpMethod::Get => {
            router.route(route.endpoint, axum::routing::get(route_handler))
        }

        g_server::route::HttpMethod::Post => {
            router.route(route.endpoint, axum::routing::post(route_handler))
        }

        g_server::route::HttpMethod::Put => {
            router.route(route.endpoint, axum::routing::put(route_handler))
        }

        g_server::route::HttpMethod::Patch => {
            router.route(route.endpoint, axum::routing::patch(route_handler))
        }

        g_server::route::HttpMethod::Head => {
            router.route(route.endpoint, axum::routing::head(route_handler))
        }

        g_server::route::HttpMethod::Query => {
            router.route(route.endpoint, axum::routing::get(route_handler))
        }

        g_server::route::HttpMethod::Any => {
            router.route(route.endpoint, axum::routing::any(route_handler))
        }
    };

    router
}

pub fn __route_app_a_handler_2(
    router: axum::Router<Context>,
    global_config: g_server::Config,
) -> axum::Router<Context> {
    let config = global_config;
    let executor = g_server::route::Executor::new(handler_2);
    // We assemble middlewares from last to first: first in array execute first, so we declare last here to make it executed first.
    // We assemble these middlewares directly from macro declaration.
    // If no middlewares, straight to route.
    let executor = g_server::route::Executor::new(move |cx, req| logger(cx, req, executor));
    let executor = g_server::route::Executor::new(move |cx, req| auth(cx, req, executor));
    let route = g_server::route::Route::<_> {
        method: g_server::route::HttpMethod::Post,
        endpoint: "/route_2",
        config,
        response_body_type: g_server::route::ResponseBodyType::Json,
        executor: executor,
    };

    #[rustfmt::skip]
    let route_handler = move |
          axum::extract::State(cx): axum::extract::State<Context>,
          headers: axum::http::HeaderMap,
          axum::extract::Path(path_params): axum::extract::Path<()>,
          axum::extract::Query(query_params): axum::extract::Query<()>,
          axum::extract::Json(body): axum::extract::Json<RequestBody_2>,
    | async move {
        let req = g_server::Request {
            headers,
            path_params,
            query_params,
            body,
        };

        route.executor.exec(cx, req).await.into_json_response()
    };

    let router = match route.method {
        g_server::route::HttpMethod::Get => {
            router.route(route.endpoint, axum::routing::get(route_handler))
        }

        g_server::route::HttpMethod::Post => {
            router.route(route.endpoint, axum::routing::post(route_handler))
        }

        g_server::route::HttpMethod::Put => {
            router.route(route.endpoint, axum::routing::put(route_handler))
        }

        g_server::route::HttpMethod::Patch => {
            router.route(route.endpoint, axum::routing::patch(route_handler))
        }

        g_server::route::HttpMethod::Head => {
            router.route(route.endpoint, axum::routing::head(route_handler))
        }

        g_server::route::HttpMethod::Query => {
            router.route(route.endpoint, axum::routing::get(route_handler))
        }

        g_server::route::HttpMethod::Any => {
            router.route(route.endpoint, axum::routing::any(route_handler))
        }
    };

    router
}
