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
		},

		app_config: ConfigType, // struct containing all configs from env vars, static, this will be accessible by all.
		
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
struct Request<PathParamsType, QueryParamsType, BodyType> {
	header: HeaderMap, // depend on implementation, but lets just use axum's HeaderMap for now. 
	path_params: Option<PathParamsType>,
	query_params: Option<QueryParamsType>,
	body: Option<BodyType>,
	body_str: Option<String>,
}

// response type:
struct Response<BodyType> {
	status: Status, // we use our existing status type.
	header: HeaderType, // depend on implementation, but lets just use axum's HeaderMap for now. 
	body: Option<BodyType>,	
}

// trait for polymorphic response, we support:
// - (status_code, Header, Body)
// - (status_code, Header, ()) // no body
// - (status_code, (), ()) // no header nor body
// - (status_code, Header, String) // string body
// - (status_code, (), String) // string body without header
trait IntoResponse {
	fn into_response(self) -> Response<BodyType>; 
}

// application context/state
#[derive(Clone)]
struct AppContext<C> {
	context: C
}
C: Send + Sync + Clone + 'static' // --> typical dependencies, Arc, static, and alike.

// handler signature
async handler(cx: AppContext, req: Request<...>) -> impl IntoResponse {
	...
}## Request & Response
```
