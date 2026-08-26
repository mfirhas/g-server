# g-server

`g-server` provides a generic and uniform interface for building HTTP, SSE, WebSocket, and MCP servers.

`g-server` will generate the entire entry point for the server, including main().

```rust
gserver! {
	// server declaration, with name, ip and port(u16).
	// OPTIONAL: Any kind of servers are optional. If none declared, eprintln!("g-server: no servers registered");
	http("app_name", "0.0.0.0", 42369) {
		// server configs, top level one applies to all endpoints inside this server.
		// OPTIONAL: config is optional and fields are also optional. 
		// The way config initializes is first it calls `::default`, and then set these fields.
		config {
			timeout: 5000, // timeout for all handlers under this banner, in ms, default: 5000 ms
			concurrency_limit: 100_000, // limits number of concurrent requests, default: 100,000
			body_limit: 10240, // request body max size in KiB, default: 10240 KiB (10 MiB)
			compression: All, // compression methods: Deflate, Gzip, Brotli, Zstd. Default: All: Zstd, Brotli, Gzip, Deflate.
			keep_alive: 1000, // max time an idle connection stays open, in ms, default: 1000 ms
			...
		}, // Config for server, optional, default to something.

		// OPTIONAL: if omitted, context is ().
		app_context: ContextType, // struct containing all app's context, such as configs and dependencies. Accessible to all endpoints. Must have `init()` method.

		// group of endpoints shared by same prefix
		group {
			// MANDATORY: will be prepended on every members path
			prefix: "/prefix",
			// OPTIONAL: these config will overrides global config.
			config: {
				... // config for this group
			},
			// OPTIONAL
			middlewares: [
				...
			], // middlewares
			// MANDATORY: a group must have at least 1 member
			members: [
				get {
					...
				},
				post {
					...
				},
				... // just like non-group endpoints.
				// same rules applies
				group {
					... // support nested group.
				},
			]
		},
		
		// endpoints
		// we must detect duplicate method and endpoint at compile time.
		// method: head, get, post, query, etc
		get { 
			// MANDATORY: define this route path.
			endpoint: "/the/endpoint", // can be literal or from static, mandatory
			// OPTIONAL: these config will overrides global config.
			config: {
				... // config for this endpoint.
			},
			// OPTIONAL: If omitted, it becomes (): Path<()>
			path_params: PathParamsType, // a struct containing all request path params, compile time construct from endpoint, /endpoint/:id/:code/:name -> Struct {id: i32, code: String, name: String } -> might failed at construction 
			// OPTIONAL: If omitted, it becomes (): Query<()>
			query_params: QueryParamsType, // a struct containing all request queries `?q1=a&q2=b`, optional.
			// OPTIONAL: If omitted, the body is ().
			// Comes in 3 kinds:
			// - String -> body is string
			// - Json(StructType) -> body is json: Json<StructType>
			// - Form(StructType) -> accepting x-www-form-urlencoded: Form<StructType>
			request_body: BodyType, // for url-encoded form, use Form(Struct), a struct containing request body type, optional.
			// OPTIONAL
			middlewares: [
				middleware1_fn,
				middleware2_fn,
				middleware3_fn,
			], // middlewares
			// MANDATORY: each route must have handler
			handler: FunctionName, // function pointer to handler type we define, and translate to implementation handler, mandatory
			// OPTIONAL: if omitted, default to Json.
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
