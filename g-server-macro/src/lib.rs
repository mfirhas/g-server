#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::{collections::HashSet, str::FromStr};
use syn::{
    Expr, LitInt, LitStr, Path, Result, Token, Type, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

mod config;
mod response_body;

#[proc_macro]
pub fn gserver(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as GServer);

    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

// ============================================================
// AST
// ============================================================

pub(crate) struct GServer {
    pub(crate) servers: Vec<Server>,
}

pub(crate) struct Server {
    pub(crate) kind: ServerKind,
    pub(crate) name: LitStr,
    pub(crate) ip: LitStr,
    pub(crate) port: LitInt,
    pub(crate) body: ServerBody,
}

pub(crate) enum ServerKind {
    Http,
    Sse,
    Ws,
    Mcp,
}

pub(crate) struct ServerBody {
    pub(crate) config: Vec<crate::config::ConfigEntry>,

    // OPTIONAL:
    // If omitted, context is ().
    pub(crate) context: Option<Type>,

    pub(crate) routes: Vec<Route>,
}

pub(crate) struct Route {
    pub(crate) method: HttpMethod,

    // MANDATORY.
    //
    // Kept as an expression so this can eventually support:
    //
    // endpoint: "/foo",
    // endpoint: SOME_STATIC,
    //
    // instead of only a string literal.
    pub(crate) endpoint: Expr,

    // OPTIONAL.
    pub(crate) config: Vec<crate::config::ConfigEntry>,

    // OPTIONAL.
    // If omitted => Path<()>.
    pub(crate) path_params: Option<Type>,

    // OPTIONAL.
    // If omitted => Query<()>.
    pub(crate) query_params: Option<Type>,

    // OPTIONAL.
    // If omitted => body ().
    pub(crate) request_body: Option<RequestBody>,

    // OPTIONAL.
    pub(crate) middlewares: Vec<Path>,

    // MANDATORY.
    pub(crate) handler: Path,

    // OPTIONAL.
    // Defaults to Json.
    pub(crate) response_body: crate::response_body::ResponseBody,
}

pub(crate) enum RequestBody {
    // Json(StructType)
    Json(Type),

    // Form(StructType)
    Form(Type),

    // String
    String,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Options,
    Head,
    Trace,
    Query,
    Any,
}

// ============================================================
// Parsing
//
// Parse `gserver!` body:
// ```rust,ignore
// gserver! {
//     <body> // parse this
// }
// ```
// ============================================================

impl Parse for GServer {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut servers = Vec::new();

        while !input.is_empty() {
            let kind_ident: Ident = input.parse()?;

            let kind = match kind_ident.to_string().as_str() {
                "http" => ServerKind::Http,
                "sse" => ServerKind::Sse,
                "ws" => ServerKind::Ws,
                "mcp" => ServerKind::Mcp,

                _ => {
                    return Err(syn::Error::new(
                        kind_ident.span(),
                        "expected `http`, `sse`, `ws`, or `mcp`",
                    ));
                }
            };

            let content;
            syn::parenthesized!(content in input);

            let name: LitStr = content.parse()?;

            content.parse::<Token![,]>()?;

            let ip: LitStr = content.parse()?;

            content.parse::<Token![,]>()?;

            let port: LitInt = content.parse()?;

            if !content.is_empty() {
                return Err(content.error("expected server declaration: name, ip, port"));
            }

            let body;
            braced!(body in input);

            let body = parse_server_body(&body)?;

            consume_comma(input)?;

            servers.push(Server {
                kind,
                name,
                ip,
                port,
                body,
            });
        }

        Ok(Self { servers })
    }
}

// ============================================================
// Server body
// ============================================================

fn parse_server_body(input: ParseStream<'_>) -> Result<ServerBody> {
    let mut config = Vec::new();
    let mut context = None;
    let mut routes = Vec::new();

    while !input.is_empty() {
        let key: Ident = input.parse()?;

        match key.to_string().as_str() {
            // OPTIONAL.
            "config" => {
                let content;

                braced!(content in input);

                config = crate::config::parse_config(&content)?;
            }

            // OPTIONAL.
            //
            // If omitted, generated context is ().
            "app_context" => {
                input.parse::<Token![:]>()?;

                context = Some(input.parse()?);
            }

            // GROUPS are part of the DSL plan but are intentionally
            // not implemented in this route-first implementation yet.
            "group" => {
                return Err(syn::Error::new(
                    key.span(),
                    "`group` is not implemented yet",
                ));
            }

            // Routes.
            "get" => {
                routes.push(parse_route(input, HttpMethod::Get)?);
            }

            "post" => {
                routes.push(parse_route(input, HttpMethod::Post)?);
            }

            "put" => {
                routes.push(parse_route(input, HttpMethod::Put)?);
            }

            "patch" => {
                routes.push(parse_route(input, HttpMethod::Patch)?);
            }

            "delete" => {
                routes.push(parse_route(input, HttpMethod::Delete)?);
            }

            "options" => {
                routes.push(parse_route(input, HttpMethod::Options)?);
            }

            "head" => {
                routes.push(parse_route(input, HttpMethod::Head)?);
            }

            "trace" => {
                routes.push(parse_route(input, HttpMethod::Trace)?);
            }

            "query" => {
                routes.push(parse_route(input, HttpMethod::Query)?);
            }

            "any" => {
                routes.push(parse_route(input, HttpMethod::Any)?);
            }

            _ => {
                return Err(syn::Error::new(key.span(), "unexpected server member"));
            }
        }

        consume_comma(input)?;
    }

    Ok(ServerBody {
        config,
        context,
        routes,
    })
}

// ============================================================
// Route
// ============================================================

fn parse_route(input: ParseStream<'_>, method: HttpMethod) -> Result<Route> {
    let content;

    braced!(content in input);

    // OPTIONAL fields start with their defaults.
    let mut endpoint = None;
    let mut config = Vec::new();
    let mut path_params = None;
    let mut query_params = None;
    let mut request_body = None;
    let mut middlewares = Vec::new();

    // MANDATORY, therefore remains None until parsed.
    let mut handler = None;

    // OPTIONAL, defaults to Json.
    let mut response_body = crate::response_body::ResponseBody::default();

    while !content.is_empty() {
        let key: Ident = content.parse()?;

        match key.to_string().as_str() {
            // MANDATORY.
            "endpoint" => {
                content.parse::<Token![:]>()?;

                endpoint = Some(content.parse()?);
            }

            // OPTIONAL.
            "config" => {
                content.parse::<Token![:]>()?;

                let body;

                braced!(body in content);

                config = crate::config::parse_config(&body)?;
            }

            // OPTIONAL.
            //
            // Omitted => Path<()>.
            "path_params" => {
                content.parse::<Token![:]>()?;

                path_params = Some(content.parse()?);
            }

            // OPTIONAL.
            //
            // Omitted => Query<()>.
            "query_params" => {
                content.parse::<Token![:]>()?;

                query_params = Some(content.parse()?);
            }

            // OPTIONAL.
            //
            // Supported:
            //
            // request_body: String
            // request_body: Json(MyStruct)
            // request_body: Form(MyStruct)
            "request_body" => {
                content.parse::<Token![:]>()?;

                request_body = Some(parse_request_body(&content)?);
            }

            // OPTIONAL.
            "middlewares" => {
                content.parse::<Token![:]>()?;

                let body;

                syn::bracketed!(body in content);

                while !body.is_empty() {
                    middlewares.push(body.parse()?);

                    consume_comma(&body)?;
                }
            }

            // MANDATORY.
            "handler" => {
                content.parse::<Token![:]>()?;

                handler = Some(content.parse()?);
            }

            // OPTIONAL.
            //
            // Default = Json.
            "response_body" => {
                content.parse::<Token![:]>()?;

                let ident: Ident = content.parse()?;

                response_body =
                    crate::response_body::ResponseBody::from_str(ident.to_string().as_str())
                        .map_err(|err| syn::Error::new(key.span(), err))?;
            }

            _ => {
                return Err(syn::Error::new(key.span(), "unexpected route member"));
            }
        }

        consume_comma(&content)?;
    }

    // --------------------------------------------------------
    // Mandatory validation
    // --------------------------------------------------------

    let endpoint = endpoint.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "route requires mandatory field `endpoint`",
        )
    })?;

    let handler = handler.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "route requires mandatory field `handler`",
        )
    })?;

    Ok(Route {
        method,
        endpoint,
        config,
        path_params,
        query_params,
        request_body,
        middlewares,
        handler,
        response_body,
    })
}

// ============================================================
// Request body
// ============================================================

fn parse_request_body(input: ParseStream<'_>) -> Result<RequestBody> {
    let kind: Ident = input.parse()?;

    match kind.to_string().as_str() {
        "String" => Ok(RequestBody::String),

        "Json" => {
            let body;
            syn::parenthesized!(body in input);

            let ty: Type = body.parse()?;

            Ok(RequestBody::Json(ty))
        }

        "Form" => {
            let body;
            syn::parenthesized!(body in input);

            let ty: Type = body.parse()?;

            Ok(RequestBody::Form(ty))
        }

        _ => Err(syn::Error::new(
            kind.span(),
            "expected `String`, `Json(Type)`, or `Form(Type)`",
        )),
    }
}

pub(crate) fn consume_comma(input: ParseStream<'_>) -> Result<()> {
    if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
    }

    Ok(())
}

// ============================================================
// Expansion
// ============================================================

fn expand(input: GServer) -> Result<TokenStream2> {
    validate_servers(&input.servers)?;

    // --------------------------------------------------------
    // Servers are OPTIONAL.
    //
    // Zero servers is valid.
    // --------------------------------------------------------

    let http_servers = input
        .servers
        .iter()
        .filter(|server| matches!(server.kind, ServerKind::Http))
        .collect::<Vec<_>>();

    // --------------------------------------------------------
    // SSE / WS / MCP are recognized by the parser, but not
    // implemented yet.
    // --------------------------------------------------------

    for server in &input.servers {
        match server.kind {
            ServerKind::Http => {}

            ServerKind::Sse => {
                return Err(syn::Error::new(
                    server.name.span(),
                    "`sse` server is not implemented yet",
                ));
            }

            ServerKind::Ws => {
                return Err(syn::Error::new(
                    server.name.span(),
                    "`ws` server is not implemented yet",
                ));
            }

            ServerKind::Mcp => {
                return Err(syn::Error::new(
                    server.name.span(),
                    "`mcp` server is not implemented yet",
                ));
            }
        }
    }

    let main = generate_main(&http_servers);

    let initializers = http_servers
        .iter()
        .map(|server| generate_init_function(server))
        .collect::<Result<Vec<_>>>()?;

    let mut routes = Vec::new();

    for server in &http_servers {
        for (index, route) in server.body.routes.iter().enumerate() {
            routes.push(generate_route_function(server, route, index)?);
        }
    }

    Ok(quote! {
        // These traits are needed by generated response
        // conversion calls.
        use g_server::{
            IntoAxumHtmlResponse,
            IntoAxumJsonResponse,
            IntoAxumStringResponse,
        };

        #main

        #(#initializers)*

        #(#routes)*
    })
}

// ============================================================
// Validation
// ============================================================

fn validate_servers(servers: &[Server]) -> Result<()> {
    let mut names = HashSet::new();
    let mut ports = HashSet::new();

    for server in servers {
        // ----------------------------------------------------
        // Server name must be unique.
        // ----------------------------------------------------

        let name = server.name.value();

        if !names.insert(name.clone()) {
            return Err(syn::Error::new(
                server.name.span(),
                format!("duplicate server name `{name}`"),
            ));
        }

        // ----------------------------------------------------
        // Port must be unique.
        // ----------------------------------------------------

        let port: u16 = server.port.base10_parse()?;

        if !ports.insert(port) {
            return Err(syn::Error::new(
                server.port.span(),
                format!("duplicate server port `{port}`"),
            ));
        }

        // ----------------------------------------------------
        // HTTP-specific validation.
        // ----------------------------------------------------

        if matches!(server.kind, ServerKind::Http) {
            validate_routes(server)?;
        }
    }

    Ok(())
}

fn validate_routes(server: &Server) -> Result<()> {
    let mut routes = HashSet::new();

    for route in &server.body.routes {
        if let Expr::Lit(expr) = &route.endpoint {
            if let syn::Lit::Str(endpoint) = &expr.lit {
                let endpoint_value = endpoint.value();

                let key = (route.method, endpoint_value.clone());

                if !routes.insert(key) {
                    return Err(syn::Error::new(
                        endpoint.span(),
                        format!(
                            "duplicate route: {} {}",
                            method_name(route.method),
                            endpoint_value,
                        ),
                    ));
                }
            }
        }
    }

    Ok(())
}

// ============================================================
// main()
// ============================================================

fn generate_main(servers: &[&Server]) -> TokenStream2 {
    // --------------------------------------------------------
    // OPTIONAL: zero servers.
    //
    // The generated binary is still valid.
    // --------------------------------------------------------

    if servers.is_empty() {
        return quote! {
            #[::tokio::main]
            async fn main() {
                eprintln!(
                    "g-server: no servers registered"
                );
            }
        };
    }

    let initializers = servers.iter().map(|server| {
        let name = server_ident(server);

        let init = init_ident(server);

        quote! {
            let #name = #init();
        }
    });

    let listeners = servers.iter().map(|server| {
        let name = server_ident(server);

        let listener = format_ident!("{}_listener", name);

        quote! {
            let #listener =
                ::tokio::net::TcpListener::bind(
                    (
                        #name.0.ip_address,
                        #name.0.port
                    )
                )
                .await
                .expect(
                    format!(
                        "failed creating {} tcp listener",
                        #name.0.name
                    )
                    .as_str()
                );

            println!(
                "g-server: running {} on {}:{}...",
                #name.0.name,
                #name.0.ip_address,
                #name.0.port
            );
        }
    });

    let serves = servers.iter().map(|server| {
        let name = server_ident(server);

        let listener = format_ident!("{}_listener", name);

        quote! {
            ::axum::serve(
                #listener,
                #name.1,
            )
        }
    });

    quote! {
        #[::tokio::main]
        async fn main() {
            #(#initializers)*

            #(#listeners)*

            ::tokio::try_join!(
                #(#serves),*
            )
            .expect(
                "failed running all servers..."
            );
        }
    }
}

// ============================================================
// __init_<server>()
// ============================================================

fn generate_init_function(server: &Server) -> Result<TokenStream2> {
    let init = init_ident(server);

    let name = server.name.value();

    let ip = server.ip.value();

    let port: u16 = server.port.base10_parse()?;

    // OPTIONAL context:
    //
    // app_context: Context
    //     => Context::init()
    //
    // omitted
    //     => ()
    let context_type = server
        .body
        .context
        .as_ref()
        .map(|ty| quote!(#ty))
        .unwrap_or_else(|| quote!(()));

    let context_init = match server.body.context.as_ref() {
        Some(ty) => quote! {
            let context =
                <#ty>::init();
        },

        None => quote! {
            let context = ();
        },
    };

    let global_config = crate::config::generate_global_config(&server.body.config);

    let route_calls = server.body.routes.iter().enumerate().map(|(index, _)| {
        let route = route_function_ident(server, index);

        quote! {
            let router =
                #route(
                    router,
                    global_config.clone(),
                );
        }
    });

    Ok(quote! {
        pub fn #init() -> (
            g_server::Server,
            ::axum::Router<()>,
        ) {
            let server =
                g_server::Server {
                    name: #name,
                    ip_address: #ip,
                    port: #port,
                };

            // Override only fields explicitly supplied
            // by the user.
            #global_config

            // OPTIONAL context.
            #context_init

            let router =
                ::axum::Router::<#context_type>::new();

            #(#route_calls)*

            let router =
                router.with_state(context);

            (server, router)
        }
    })
}

// ============================================================
// __route_<server>_<handler>()
// ============================================================

fn generate_route_function(server: &Server, route: &Route, index: usize) -> Result<TokenStream2> {
    let function = route_function_ident(server, index);

    // OPTIONAL context.
    let context = server.body.context.as_ref();

    let context_ty = context.map(|ty| quote!(#ty)).unwrap_or_else(|| quote!(()));

    let handler = &route.handler;

    // OPTIONAL => Path<()>
    let path_ty = route
        .path_params
        .as_ref()
        .map(|ty| quote!(#ty))
        .unwrap_or_else(|| quote!(()));

    // OPTIONAL => Query<()>
    let query_ty = route
        .query_params
        .as_ref()
        .map(|ty| quote!(#ty))
        .unwrap_or_else(|| quote!(()));

    // OPTIONAL request body.
    let body_extractor = generate_body_extractor(&route.request_body);

    let middleware_chain = generate_middleware_chain(route, handler);

    let response_conversion = generate_response_conversion(route.response_body);

    let method = method_tokens(route.method);

    let response_body_type = format_ident!("{}", route.response_body.to_string());

    let endpoint = &route.endpoint;

    // Route config starts from inherited global
    // config and overrides only explicitly declared
    // fields.
    let route_config = crate::config::generate_route_config(&route.config);

    let registration = generate_route_registration(route.method);

    Ok(quote! {
        pub fn #function(
            router: ::axum::Router<#context_ty>,
            global_config: g_server::Config,
        ) -> ::axum::Router<#context_ty> {
            // Then override route-specific fields.
            #route_config

            // Handler + optional middleware chain.
            #middleware_chain

            let route =
                g_server::route::Route::<_> {
                    method: #method,
                    endpoint: #endpoint,
                    config,
                    response_body_type: g_server::route::ResponseBodyType::#response_body_type,
                    executor,
                };

            let route_handler = move |
                ::axum::extract::State(cx):
                    ::axum::extract::State<#context_ty>,

                headers: ::axum::http::HeaderMap,

                ::axum::extract::Path(path_params):
                    ::axum::extract::Path<#path_ty>,

                ::axum::extract::Query(query_params):
                    ::axum::extract::Query<#query_ty>,

                #body_extractor
            | async move {
                let req = g_server::Request {
                    headers,
                    path_params,
                    query_params,
                    body,
                };

                route
                    .executor
                    .exec(cx, req)
                    .await
                    #response_conversion
            };

            #registration
        }
    })
}

// ============================================================
// Middleware chain
// ============================================================

fn generate_middleware_chain(route: &Route, handler: &Path) -> TokenStream2 {
    // No middleware:
    //
    // Executor::new(handler)
    //
    // Middleware:
    //
    // handler
    //   ↓
    // middleware3
    //   ↓
    // middleware2
    //   ↓
    // middleware1
    //
    // Therefore declarations are wrapped in reverse order.

    let mut output = quote! {
        let executor =
            g_server::route::Executor::new(
                #handler
            );
    };

    for middleware in route.middlewares.iter().rev() {
        output.extend(quote! {
            let executor =
                g_server::route::Executor::new(
                    move |cx, req| {
                        #middleware(
                            cx,
                            req,
                            executor,
                        )
                    }
                );
        });
    }

    output
}

// ============================================================
// Request body
// ============================================================

fn generate_body_extractor(body: &Option<RequestBody>) -> TokenStream2 {
    match body {
        // JSON body:
        //
        // request_body: Json(MyStruct)
        Some(RequestBody::Json(ty)) => {
            quote! {
                ::axum::extract::Json(body):
                    ::axum::extract::Json<#ty>,
            }
        }

        // Form body:
        //
        // request_body: Form(MyStruct)
        Some(RequestBody::Form(ty)) => {
            quote! {
                ::axum::extract::Form(body):
                    ::axum::extract::Form<#ty>,
            }
        }

        // String body:
        //
        // request_body: String
        Some(RequestBody::String) => {
            quote! {
                body: String,
            }
        }

        // No request_body:
        //
        // body is simply ().
        None => {
            quote! {
                body: (),
            }
        }
    }
}

// ============================================================
// Response conversion
// ============================================================

fn generate_response_conversion(body: crate::response_body::ResponseBody) -> TokenStream2 {
    match body {
        crate::response_body::ResponseBody::Json => {
            quote! {
                .into_json_response()
            }
        }

        crate::response_body::ResponseBody::String => {
            quote! {
                .into_string_response()
            }
        }

        crate::response_body::ResponseBody::Html => {
            quote! {
                .into_html_response()
            }
        }
    }
}

// ============================================================
// Axum routing
// ============================================================

fn generate_route_registration(method: HttpMethod) -> TokenStream2 {
    match method {
        HttpMethod::Get => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::get(
                        route_handler
                    ),
                )
            }
        }

        HttpMethod::Post => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::post(
                        route_handler
                    ),
                )
            }
        }

        HttpMethod::Put => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::put(
                        route_handler
                    ),
                )
            }
        }

        HttpMethod::Patch => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::patch(
                        route_handler
                    ),
                )
            }
        }

        HttpMethod::Delete => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::delete(
                        route_handler
                    ),
                )
            }
        }

        HttpMethod::Options => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::options(
                        route_handler
                    ),
                )
            }
        }

        HttpMethod::Head => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::head(
                        route_handler
                    ),
                )
            }
        }

        HttpMethod::Trace => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::trace(
                        route_handler
                    ),
                )
            }
        }

        // Current DSL semantics:
        // `query` is represented by GET at the Axum layer.
        HttpMethod::Query => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::get(
                        route_handler
                    ),
                )
            }
        }

        HttpMethod::Any => {
            quote! {
                router.route(
                    route.endpoint,
                    ::axum::routing::any(
                        route_handler
                    ),
                )
            }
        }
    }
}

// ============================================================
// HttpMethod
// ============================================================

fn method_tokens(method: HttpMethod) -> TokenStream2 {
    let ident = match method {
        HttpMethod::Get => {
            format_ident!("Get")
        }

        HttpMethod::Post => {
            format_ident!("Post")
        }

        HttpMethod::Put => {
            format_ident!("Put")
        }

        HttpMethod::Patch => {
            format_ident!("Patch")
        }

        HttpMethod::Delete => {
            format_ident!("Delete")
        }

        HttpMethod::Options => {
            format_ident!("Options")
        }

        HttpMethod::Head => {
            format_ident!("Head")
        }

        HttpMethod::Trace => {
            format_ident!("Trace")
        }

        HttpMethod::Query => {
            format_ident!("Query")
        }

        HttpMethod::Any => {
            format_ident!("Any")
        }
    };

    quote! {
        g_server::route::HttpMethod::#ident
    }
}

// ============================================================
// Identifiers
// ============================================================

fn server_ident(server: &Server) -> Ident {
    // NOTE:
    //
    // This currently assumes the server name can be used as
    // a Rust identifier:
    //
    // http("app_a", ...)
    //
    // Later we should decouple the user-facing server name
    // from generated Rust identifiers so names like
    // "my-api" are also valid.
    Ident::new(&server.name.value(), server.name.span())
}

fn init_ident(server: &Server) -> Ident {
    format_ident!("__init_{}", server.name.value())
}

fn route_function_ident(server: &Server, index: usize) -> Ident {
    let handler_name = server
        .body
        .routes
        .get(index)
        .and_then(|route| route.handler.segments.last())
        .map(|segment| segment.ident.to_string())
        .unwrap_or_else(|| format!("route_{index}"));

    format_ident!("__route_{}_{}", server.name.value(), handler_name)
}

// ============================================================
// Method name for diagnostics
// ============================================================

fn method_name(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Options => "OPTIONS",
        HttpMethod::Head => "HEAD",
        HttpMethod::Trace => "TRACE",
        HttpMethod::Query => "QUERY",
        HttpMethod::Any => "ANY",
    }
}
