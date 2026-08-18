# g-server

`g-server` provides a generic and uniform interface for building HTTP, SSE, WebSocket, and MCP servers.

`g-server` will generate the entire entry point for the server, including main().

```rust
gserver! {
	http("app_name") {
		// server config, ip, ports, and other http server configs.
		config {
			ip: "0.0.0.0",
			port: 9999, // u16
			global_timeout: 5000, // timeout for all handlers under this banner, in ms
			...
		}, // Config for server, optional, default to something.

		app_context: ContextType, // struct containing all app's context, such as configs and dependencies. Accessible to all endpoints. Must contains `init()` method.

		// group of endpoints shared by same prefix
		group {
			prefix: "/prefix",
			pre: [
				...
			], // pre middlewares applied to all members of this group.
			post: [
				...
			], // post middlewares applied to all members of this group.
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
			path_params: PathParamsType, // a struct containing all request path params, compile time construct from endpoint, /endpoint/:id/:code/:name -> Struct {id: i32, code: String, name: String } -> might failed at construction 
			query_params: QueryParamsType, // a struct containing all request queries `?q1=a&q2=b`, optional
			body: BodyType, // a struct containing request body type, optional
			pre: [
				middleware1_fn,
				middleware2_fn,
				middleware3_fn,
			], // middleware before hitting handler
			post: [
				middleware1_fn,
				middleware2_fn,	
			], // middleware after handler returned
			handler: FunctionName, // function pointer to handler type we define, and translate to implementation handler, mandatory
		},

		post {
			...
		},

		put {
			...
		},

		// latest http verb added
		query {
			...
		},

		// any method
		any {
			...
		}
	},

	http("another_app_in_different_port") {
		... // same settings like above
	},

	sse("sse server name") {
		// TBD
	},

	ws("web socket server name") {
		// TBD
	},

	mcp("mcp server name") {
		// TBD
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

// application context/state
#[derive(Clone)]
pub struct AppContext<C> where C: Clone + Send + Sync + 'static'{
	pub name: &'static str, // app's name, gotten from server's name: http("app_name")
	pub context: C, // empty -> ()
} // --> typical dependencies, Arc, static, and alike, cheap to clone.

// handler signature
// P, Q, B are concretely defined.
pub async fn handler(cx: AppContext<ContextType>, req: Request<P,Q,B>) -> Result<Response<BodyType>, Response<ErrorType>> {
	...
}

// pre middleware signature
// P, Q, B are concretely defined.
pub async fn pre_middleware(cx: AppContext<ContextType>, req: Request<P,Q,B>) -> Result<Request<P,Q,B> , Response<ErrorType>> {
	...
}

// post middleware signature
// B is concretely defined.
pub async fn post_middleware(cx: AppContext<ContextType>, res: Response<B>) -> Result<Response<B> , Response<ErrorType>> {
	...
}
```

## axum handler
AppContext is passed through axum's state.

Route name is `__route_<handler_name>`.
```rust
async fn __route_handler_name(...) -> Result<BodyType, ErrorType> {
	let req: Request<...> = ... // converting axum's request data into `Request<PathParamsType, QueryParamsType, BodyType>`

	let req = pre_middleware1(cx, req).await?;
	let req = pre_middleware2(cx, req).await?;
	let req = pre_middleware3(cx, req).await?;
	// ...

	let res = handler(cx, req).await?;

	let res = post_middleware1(cx, res).await?;
	let res = post_middleware2(cx, res).await?;
	let res = post_middleware3(cx, res).await?;
	// ...

	Ok(res)
}
```
