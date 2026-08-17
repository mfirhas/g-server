# g-server

`g-server` provides a generic and uniform interface for building HTTP, SSE, WebSocket, and MCP servers.

```
gserver! {
	http("app_name") {
		// server config, ip, ports, and other http server configs.
		config {
			ip: "0.0.0.0",
			port: "9999",
			global_timeout: 5000, // timeout for all handlers under this banner, in ms
			...
		}, // Config for server, optional, default to something.

		app_config: ConfigType, // struct containing all configs from env vars, static, this will be accessible by all. Optional.

		// group of endpoints shared by same prefix
		group {
			prefix: "/prefix",
			pre: [
				...
			], // middlewares applied to all members of this group.
			post: [
				...
			], // middlewares applied to all members of this group.
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
			path_params: PathParamsType, // a struct containing all path params, compile time construct from endpoint, /endpoint/:id/:code/:name -> Struct {id: i32, code: String, name: String } -> might failed at construction 
			query_params: QueryParamsType, // a struct containing all queries `?q1=a&q2=b`, optional
			body: BodyType, // a struct containing json body, optional
			body_str, // if we accept string as body, optional, either body or body_str, cant be both. Optional
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
	pub path_params: PathParamsType,
	pub query_params: QueryParamsType,
	pub body: BodyType,
}

// response type:
pub struct Response<BodyType> {
	pub status: StatusCode, // use http crate StatusCode.
	pub header: HeaderMap, // http crate HeaderMap.
	pub body: Option<BodyType>,	
}

// application context/state
#[derive(Clone)]
struct AppContext<C> {
	pub context: C
}
C: Clone + Send + Sync + 'static' // --> typical dependencies, Arc, static, and alike.

// handler signature
pub async fn handler(cx: AppContext<ContextType>, req: Request<...>) -> impl Into<Response<BodyType>> {
	...
}
```
