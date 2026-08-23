# g-server

`g-server` provides a generic and uniform interface for building HTTP, SSE, WebSocket, and MCP servers.

`g-server` will generate the entire entry point for the server, including main().

```rust
gserver! {
	// server declaration, with name, ip and port(u16).
	http("app_name", "0.0.0.0", 42369) {
		// server configs, top level one applies to all endpoints inside this server.
		config {
			timeout: 5000, // timeout for all handlers under this banner, in ms, default: 5000 ms
			concurrency_limit: 100_000, // limits number of concurrent requests, default: 100,000
			body_limit: 10, // MiB, default: 10 MiB
			compression: Zstd, // compression methods: Deflate, Gzip, Brotli, Zstd. Default: Zstd
			keep_alive: 1000, // max time an idle connection stays open, in ms, default: 1000 ms
			app_context: ContextType, // struct containing all app's context, such as configs and dependencies. Accessible to all endpoints. Must have `init()` method.
			...
		}, // Config for server, optional, default to something.

		// group of endpoints shared by same prefix
		group {
			prefix: "/prefix",
			config: {
				... // config for this group
			},
			middlewares: [
				...
			], // middlewares
			members: [
				get {
					...
				},
				post {
					...
				},
				... // just like non-group endpoints.
				group {
					... // support nested group.
				},
			]
		},
		
		// endpoints
		// we must detect duplicate method and endpoint at compile time.
		// method: head, get, post, query, etc
		get { 
			endpoint: "/the/endpoint", // can be literal or from static, mandatory
			config: {
				... // config for this endpoint.
			},
			path_params: PathParamsType, // a struct containing all request path params, compile time construct from endpoint, /endpoint/:id/:code/:name -> Struct {id: i32, code: String, name: String } -> might failed at construction 
			query_params: QueryParamsType, // a struct containing all request queries `?q1=a&q2=b`, optional.
			request_body: BodyType, // for url-encoded form, use Form(Struct), a struct containing request body type, optional.
			middlewares: [
				middleware1_fn,
				middleware2_fn,
				middleware3_fn,
			], // middlewares
			handler: FunctionName, // function pointer to handler type we define, and translate to implementation handler, mandatory
			response_body: Json, // response body type: String(text/plain), Json(application/json), Html(text/html). Optional. Default: json. 
		},

		post {
			...
		},

		put {
			...
		},

		... // other routes

		// latest http verb added
		query {
			...
		},

		// any method
		any {
			...
		}
	},

	// server name and port must be unique.
	http("another_app_in_different_port", "0.0.0.0", 42469) {
		... // same settings like above
	},

	sse("sse server name", "0.0.0.0", 42569) {
		// TODO
	},

	ws("web socket server name", "0.0.0.0", 42669) {
		// TODO
	},

	mcp("mcp server name", "0.0.0.0", 42769) {
		// TODO
	}
}
```

```rust
// request type:
pub struct Request<PathParamsType, QueryParamsType, BodyType> {
	pub header: HeaderMap, // http crate HeaderMap
	pub path_params: PathParamsType, // empty -> ()
	pub query_params: QueryParamsType, // empty -> ()
	pub body: BodyType, // empty -> ()
}

// response type:
pub struct Response<BodyType> {
	pub status: StatusCode, // http crate StatusCode.
	pub header: HeaderMap, // http crate HeaderMap.
	pub body: BodyType, // empty -> ()
}

// handler signature
// P, Q, B are concretely defined.
pub async fn handler(cx: AppContextType, req: Request<P,Q,B>) -> Result<Response<BodyType>, Response<ErrorType>> {
	...
}

// accumulate all route middlewares and handler
struct Next<F> {
    next: F,
}

impl<F> Next<F> {
    fn new<Fut, VReq, VRes>(next: F) -> Self
    where
        F: FnOnce(AppContext, Request<VReq>) -> Fut,
        Fut: Future<Output = Response<VRes>>,
    {
        Self { next }
    }
}

// route executor, contains the Next<F>.
struct RouteExecutor<F> {
    func: Next<F>,
}

impl<F> RouteExecutor<F> {
    fn new(func: Next<F>) -> Self {
        RouteExecutor { func }
    }
    async fn exec<VREQ, VRES, Fut>(self, cx: AppContext, req: Request<VREQ>) -> Response<VRES>
    where
        F: FnOnce(AppContext, Request<VREQ>) -> Fut,
        Fut: Future<Output = Response<VRES>>,
    {
        (self.func.next)(cx, req).await
    }
}

async fn middleware<F, Fut, P, Q, B>(
    cx: AppContextType,
    req: Request<P, Q, B>,
    next: Next<F>,
) -> Response<VRes>
where
    F: FnOnce(AppContext, Request<VReq>) -> Fut,
    Fut: Future<Output = Response<VRes>>,
{
		// pre

    let res = (next.next)(cx, req).await;

    // post

    res
}
```

## axum handler
AppContext is passed through axum's state.

Route name is `__route_<handler_name>`.
```rust
async fn __route_handler_name(...) -> Result<BodyType, ErrorType> {
	let req: Request<...> = ... // converting axum's request data into `Request<PathParamsType, QueryParamsType, BodyType>`

	// route_executor contains all middlewares and handler.
	// all middlewares and handler executed here.
	let res = route_executor.exec(cx, req).await?;

	Ok(res)
}
```
