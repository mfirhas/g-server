# g-server

`g-server` provides a generic and uniform interface for building HTTP, SSE, WebSocket, and MCP servers.

It abstracts away the underlying server implementation so applications can interact with a consistent API regardless of the underlying protocol implementation or framework.

It consists of three layers:

1. **Macro DSL** — declarative macros for defining and configuring servers.
2. **GServer API** — the uniform, protocol-independent API exposed to users.
3. **Implementation** — the underlying server frameworks and protocol implementations, e.g. Axum. It's replacable.

```text
                         User
                          │
                          ▼
                 ┌─────────────────┐
                 │   Macro DSL     │
                 │  declarative    │
                 │     macros      │
                 └────────┬────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │   GServer API   │
                 │                 │
                 │ Server          │
                 │ Router          │
                 │ Request         │
                 │ Response        │
                 │ State           │
                 │ Middleware      │
                 │ Handler         │
                 │ SSE             │
                 │ WebSocket       │
                 │ MCP             │
                 └────────┬────────┘
                          │
                          ▼
                 ┌─────────────────┐
                 │ Implementation  │
                 │                 │
                 │ Axum            │
                 │ Hyper           │
                 │ Tokio           │
                 │ ...             │
                 └─────────────────┘
```

## GServer concepts

* **Server** — the highest-level construct representing a server and its lifecycle.
* **Router** — defines the routes and handlers exposed by a server.
* **Request** — the incoming request presented to the application.
* **Response** — the response produced by the application.
* **State** — application state shared and made available to handlers.
* **Middleware** — logic applied around request handling, such as pre-processing and post-processing.
* **Handler** — application logic invoked for a route or request.
* **SSE** — server-sent event streaming interface.
* **WebSocket** — bidirectional WebSocket communication interface.
* **MCP** — Model Context Protocol server interface.

## HTTP
Server for HTTP 1.0 and 2.0.

```
GServer
 ├── Server
 ├── Router
 ├── Request
 ├── Response
 ├── Handler
 ├── State
 └── Middleware
```

### Request & Response
```
g-server
└── GServer HTTP API
    ├── Request
    │   ├── Method
    │   ├── URI
    │   ├── Headers
    │   └── Body
    │
    └── Response
        ├── Status
        ├── Headers
        └── Body
`
