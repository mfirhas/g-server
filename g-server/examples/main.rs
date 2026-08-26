use g_server::*;

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
pub struct RequestBody2 {
    pub user_id: u64,
    pub action: u32,
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseBody2 {
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
        status: http::StatusCode::OK,
        headers: http::HeaderMap::new(),
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
    req: Request<(), (), RequestBody2>,
) -> Result<Response<ResponseBody2>, Response<ErrorResponse>> {
    Ok(Response {
        status: http::StatusCode::OK,
        headers: http::HeaderMap::new(),
        body: ResponseBody2 {
            action: req.body.action,
            token: req.body.token,
            data: cx.data,
        },
    })
}

pub async fn auth<P, Q, ReqB, ResB, F, Fut>(
    cx: Context,
    req: Request<P, Q, ReqB>,
    next: route::Executor<F>,
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
    next: route::Executor<F>,
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

// below this is macro generated for axum implementation
// -----------------------------------------------------

#[tokio::main]
async fn main() {
    // MACRO: `app_a` comes from server/app name, e.g. http("app_a", "ip address", port)
    let app_a = __init_app_a();
    // let another_app = __init_another_app();

    // MACRO: in `app_a_listener`, the `app_a...` comes from server/app_name. Same for all `app_a` occurences
    // For each server, we generate listener with name let `<server_name>_listener = ...`.
    let app_a_listener = ::tokio::net::TcpListener::bind((app_a.0.ip_address, app_a.0.port))
        .await
        .expect(format!("failed creating {} tcp listener", app_a.0.name).as_str());
    // let another_app_listener =
    //     ::tokio::net::TcpListener::bind((another_app.0.ip_address, another_app.0.port))
    //         .await
    //         .expect(format!("failed creating {} tcp listener", another_app.0.name).as_str());

    // For each server, we generate log using its server name.
    println!(
        "g-server: running {} on {}:{}...",
        app_a.0.name, app_a.0.ip_address, app_a.0.port
    );
    // println!(
    //     "g-server: running {} on {}:{}...",
    //     another_app.0.name, another_app.0.ip_address, another_app.0.port
    // );

    // We join all servers inside this
    ::tokio::try_join!(
        ::axum::serve(app_a_listener, app_a.1),
        // ::axum::serve(another_app_listener, another_app.1),
    )
    .expect("failed running the all servers...");
}

// function name comes from `__init_<server_name>`
pub fn __init_app_a() -> (Server, axum::Router<()>) {
    let server = Server {
        name: "app_a",         // from server name
        ip_address: "0.0.0.0", // from server ip address
        port: 42069,           // from server port
    };

    let global_config = Config::default();
    // if user supply any config, we define them here:
    // global_config.timeout = Some(X); // X = user defined timeout from macro
    // ...

    let context = Context::init(); // `Context` is from macro, appended with `::init()`

    let router = axum::Router::new();

    // config middlewares, we define them if they're not None.
    // if let Some(timeout) = global_config.timeout {
    //     router = router.layer(TimeoutLayer::new(Duration::from_millis(timeout.into())));
    // }

    // we register all routes here
    // We call all functions registering the router's handlers: __route_<server_name>_<handler_name>
    let router = __route_app_a_handler_1(router, global_config.clone());
    let router = __route_app_a_handler_2(router, global_config.clone());

    let router = router.with_state(context);

    (server, router)
}

// function name comes from `__route_<server_name>_<handler_name>`
pub fn __route_app_a_handler_1(
    router: axum::Router<Context>,
    global_config: Config,
) -> axum::Router<Context> {
    let config = global_config;
    // if user supply any config, we define them here:
    // global_config.timeout = // user defined timeout from macro
    // ...
    let executor = route::Executor::new(handler_1);
    // We assemble middlewares from last to first: first in array execute first, so we declare last here to make it executed first.
    // We assemble these middlewares directly from macro declaration.
    // If no middlewares, straight to route.
    // `logger` and `auth` come from list of middlewares defined in macro declaration. We register them in reverse order, meaning first in list executed first, declare last here.
    let executor = route::Executor::new(move |cx, req| logger(cx, req, executor));
    let executor = route::Executor::new(move |cx, req| auth(cx, req, executor));
    let route = route::Route::<_> {
        method: route::HttpMethod::Post, // from macro: route::HttpMethod::$expr -> method
        endpoint: "/route_1/{user_id}/{user_email}",
        config,
        response_body_type: route::ResponseBodyType::Json, // from macro. route::ResponseBodyType::$expr
        executor: executor,
    };

    #[rustfmt::skip]
    let route_handler = move |
          axum::extract::State(cx): axum::extract::State<Context>, // `Context` comes from macro
          headers: http::HeaderMap,
          axum::extract::Path(path_params): axum::extract::Path<PathParams>, // `PathParams` comes from macro, if omitted becomes ()
          axum::extract::Query(query_params): axum::extract::Query<QueryParams>, // `QueryParams` comes from macro, if omitted becomes ()
          axum::extract::Json(body): axum::extract::Json<RequestBody>, // `RequestBody` comes from macro, if omitted becomes ()
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
        route::HttpMethod::Get => router.route(route.endpoint, axum::routing::get(route_handler)),

        route::HttpMethod::Post => router.route(route.endpoint, axum::routing::post(route_handler)),

        route::HttpMethod::Put => router.route(route.endpoint, axum::routing::put(route_handler)),

        route::HttpMethod::Patch => {
            router.route(route.endpoint, axum::routing::patch(route_handler))
        }

        route::HttpMethod::Head => router.route(route.endpoint, axum::routing::head(route_handler)),

        route::HttpMethod::Query => router.route(route.endpoint, axum::routing::get(route_handler)),

        route::HttpMethod::Any => router.route(route.endpoint, axum::routing::any(route_handler)),
    };

    router
}

// another handler within same server,
// same rules applied.
pub fn __route_app_a_handler_2(
    router: axum::Router<Context>,
    global_config: Config,
) -> axum::Router<Context> {
    let config = global_config;
    let executor = route::Executor::new(handler_2);
    // We assemble middlewares from last to first: first in array execute first, so we declare last here to make it executed first.
    // We assemble these middlewares directly from macro declaration.
    // If no middlewares, straight to route.
    let executor = route::Executor::new(move |cx, req| logger(cx, req, executor));
    let executor = route::Executor::new(move |cx, req| auth(cx, req, executor));
    let route = route::Route::<_> {
        method: route::HttpMethod::Post,
        endpoint: "/route_2",
        config,
        response_body_type: route::ResponseBodyType::Json,
        executor: executor,
    };

    #[rustfmt::skip]
    let route_handler = move |
          axum::extract::State(cx): axum::extract::State<Context>,
          headers: axum::http::HeaderMap,
          axum::extract::Path(path_params): axum::extract::Path<()>,
          axum::extract::Query(query_params): axum::extract::Query<()>,
          axum::extract::Json(body): axum::extract::Json<RequestBody2>,
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
        route::HttpMethod::Get => router.route(route.endpoint, axum::routing::get(route_handler)),

        route::HttpMethod::Post => router.route(route.endpoint, axum::routing::post(route_handler)),

        route::HttpMethod::Put => router.route(route.endpoint, axum::routing::put(route_handler)),

        route::HttpMethod::Patch => {
            router.route(route.endpoint, axum::routing::patch(route_handler))
        }

        route::HttpMethod::Head => router.route(route.endpoint, axum::routing::head(route_handler)),

        route::HttpMethod::Query => router.route(route.endpoint, axum::routing::get(route_handler)),

        route::HttpMethod::Any => router.route(route.endpoint, axum::routing::any(route_handler)),
    };

    router
}
