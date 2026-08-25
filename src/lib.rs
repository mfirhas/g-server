#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{
    Expr, LitInt, LitStr, Path, Result, Token, Type, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

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

struct GServer {
    servers: Vec<Server>,
}

struct Server {
    kind: ServerKind,
    name: LitStr,
    ip: LitStr,
    port: LitInt,
    body: ServerBody,
}

enum ServerKind {
    Http,
    Sse,
    Ws,
    Mcp,
}

struct ServerBody {
    config: Vec<ConfigEntry>,
    context: Option<Type>,
    routes: Vec<Route>,
}

struct ConfigEntry {
    name: Ident,
    value: Expr,
}

struct Route {
    method: HttpMethod,
    endpoint: LitStr,
    config: Vec<ConfigEntry>,
    path_params: Option<Type>,
    query_params: Option<Type>,
    request_body: Option<RequestBody>,
    middlewares: Vec<Path>,
    handler: Path,
    response_body: ResponseBody,
}

enum RequestBody {
    Json(Type),
    Form(Type),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum HttpMethod {
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

#[derive(Clone, Copy)]
enum ResponseBody {
    Json,
    String,
    Html,
}

// ============================================================
// Parsing
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

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else if !input.is_empty() {
                return Err(input.error("expected `,` between server declarations"));
            }

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

fn parse_server_body(input: ParseStream<'_>) -> Result<ServerBody> {
    let mut config = Vec::new();
    let mut context = None;
    let mut routes = Vec::new();

    while !input.is_empty() {
        let key: Ident = input.parse()?;

        match key.to_string().as_str() {
            "config" => {
                let content;
                braced!(content in input);
                config = parse_config(&content)?;
            }

            "app_context" => {
                input.parse::<Token![:]>()?;
                context = Some(input.parse()?);
            }

            "group" => {
                return Err(syn::Error::new(
                    key.span(),
                    "`group` is not implemented yet",
                ));
            }

            "get" => routes.push(parse_route(input, HttpMethod::Get)?),
            "post" => routes.push(parse_route(input, HttpMethod::Post)?),
            "put" => routes.push(parse_route(input, HttpMethod::Put)?),
            "patch" => routes.push(parse_route(input, HttpMethod::Patch)?),
            "delete" => routes.push(parse_route(input, HttpMethod::Delete)?),
            "options" => routes.push(parse_route(input, HttpMethod::Options)?),
            "head" => routes.push(parse_route(input, HttpMethod::Head)?),
            "trace" => routes.push(parse_route(input, HttpMethod::Trace)?),
            "query" => routes.push(parse_route(input, HttpMethod::Query)?),
            "any" => routes.push(parse_route(input, HttpMethod::Any)?),

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

fn parse_route(input: ParseStream<'_>, method: HttpMethod) -> Result<Route> {
    let content;
    braced!(content in input);

    let mut endpoint = None;
    let mut config = Vec::new();
    let mut path_params = None;
    let mut query_params = None;
    let mut request_body = None;
    let mut middlewares = Vec::new();
    let mut handler = None;
    let mut response_body = ResponseBody::Json;

    while !content.is_empty() {
        let key: Ident = content.parse()?;

        match key.to_string().as_str() {
            "endpoint" => {
                content.parse::<Token![:]>()?;
                endpoint = Some(content.parse()?);
            }

            "config" => {
                content.parse::<Token![:]>()?;

                let body;
                braced!(body in content);

                config = parse_config(&body)?;
            }

            "path_params" => {
                content.parse::<Token![:]>()?;
                path_params = Some(content.parse()?);
            }

            "query_params" => {
                content.parse::<Token![:]>()?;
                query_params = Some(content.parse()?);
            }

            "request_body" => {
                content.parse::<Token![:]>()?;
                request_body = Some(parse_request_body(&content)?);
            }

            "middlewares" => {
                content.parse::<Token![:]>()?;

                let body;
                syn::bracketed!(body in content);

                while !body.is_empty() {
                    middlewares.push(body.parse()?);
                    consume_comma(&body)?;
                }
            }

            "handler" => {
                content.parse::<Token![:]>()?;
                handler = Some(content.parse()?);
            }

            "response_body" => {
                content.parse::<Token![:]>()?;

                let ident: Ident = content.parse()?;

                response_body = match ident.to_string().as_str() {
                    "Json" => ResponseBody::Json,
                    "String" | "Text" => ResponseBody::String,
                    "Html" => ResponseBody::Html,
                    _ => {
                        return Err(syn::Error::new(
                            ident.span(),
                            "expected `Json`, `String`, or `Html`",
                        ));
                    }
                };
            }

            _ => {
                return Err(syn::Error::new(key.span(), "unexpected route member"));
            }
        }

        consume_comma(&content)?;
    }

    let endpoint =
        endpoint.ok_or_else(|| syn::Error::new(Span::call_site(), "route requires `endpoint`"))?;

    let handler =
        handler.ok_or_else(|| syn::Error::new(Span::call_site(), "route requires `handler`"))?;

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

fn parse_request_body(input: ParseStream<'_>) -> Result<RequestBody> {
    let kind: Ident = input.parse()?;

    let body;
    syn::parenthesized!(body in input);

    let ty: Type = body.parse()?;

    match kind.to_string().as_str() {
        "Json" => Ok(RequestBody::Json(ty)),
        "Form" => Ok(RequestBody::Form(ty)),
        _ => Err(syn::Error::new(
            kind.span(),
            "expected `Json(Type)` or `Form(Type)`",
        )),
    }
}

fn parse_config(input: ParseStream<'_>) -> Result<Vec<ConfigEntry>> {
    let mut entries = Vec::new();

    while !input.is_empty() {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;

        let value: Expr = input.parse()?;

        entries.push(ConfigEntry { name, value });

        consume_comma(input)?;
    }

    Ok(entries)
}

fn consume_comma(input: ParseStream<'_>) -> Result<()> {
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

    let http_servers = input
        .servers
        .iter()
        .filter(|server| matches!(server.kind, ServerKind::Http))
        .collect::<Vec<_>>();

    if http_servers.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "at least one `http` server is required",
        ));
    }

    for server in &input.servers {
        match server.kind {
            ServerKind::Http => {}

            ServerKind::Sse => {
                return Err(syn::Error::new(
                    server.name.span(),
                    "`sse` is reserved but not implemented yet",
                ));
            }

            ServerKind::Ws => {
                return Err(syn::Error::new(
                    server.name.span(),
                    "`ws` is reserved but not implemented yet",
                ));
            }

            ServerKind::Mcp => {
                return Err(syn::Error::new(
                    server.name.span(),
                    "`mcp` is reserved but not implemented yet",
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
        let name = server.name.value();

        if !names.insert(name.clone()) {
            return Err(syn::Error::new(
                server.name.span(),
                format!("duplicate server name `{name}`"),
            ));
        }

        let port: u16 = server.port.base10_parse()?;

        if !ports.insert(port) {
            return Err(syn::Error::new(
                server.port.span(),
                format!("duplicate server port `{port}`"),
            ));
        }

        if matches!(server.kind, ServerKind::Http) {
            validate_routes(server)?;
        }
    }

    Ok(())
}

fn validate_routes(server: &Server) -> Result<()> {
    let mut routes = HashSet::new();

    for route in &server.body.routes {
        let endpoint = route.endpoint.value();
        let key = (route.method, endpoint.clone());

        if !routes.insert(key) {
            return Err(syn::Error::new(
                route.endpoint.span(),
                format!(
                    "duplicate route: {} {}",
                    method_name(route.method),
                    endpoint
                ),
            ));
        }
    }

    Ok(())
}

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

// ============================================================
// main()
// ============================================================

fn generate_main(servers: &[&Server]) -> TokenStream2 {
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
                    (#name.0.ip_address, #name.0.port)
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
            ::axum::serve(#listener, #name.1)
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
            .expect("failed running all servers...");
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

    let context = server.body.context.as_ref().ok_or_else(|| {
        syn::Error::new(
            server.name.span(),
            "HTTP server requires `app_context: Type`",
        )
    })?;

    let global_config = generate_global_config(&server.body.config);

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
            let server = g_server::Server {
                name: #name,
                ip_address: #ip,
                port: #port,
            };

            #global_config

            let context =
                <#context>::init();

            let router =
                ::axum::Router::new();

            let mut router = router;

            #(#route_calls)*

            let router =
                router.with_state(context);

            (server, router)
        }
    })
}

fn generate_global_config(entries: &[ConfigEntry]) -> TokenStream2 {
    let assignments = entries.iter().map(|entry| {
        let field = &entry.name;
        let value = &entry.value;

        quote! {
            global_config.#field =
                (#value).into();
        }
    });

    quote! {
        let mut global_config =
            g_server::Config::default();

        #(#assignments)*
    }
}

// ============================================================
// __route_<server>_<handler>()
// ============================================================

fn generate_route_function(server: &Server, route: &Route, index: usize) -> Result<TokenStream2> {
    let function = route_function_ident(server, index);

    let context = server.body.context.as_ref().ok_or_else(|| {
        syn::Error::new(
            server.name.span(),
            "HTTP server requires `app_context: Type`",
        )
    })?;

    let handler = &route.handler;

    let path_ty = route
        .path_params
        .as_ref()
        .map(|ty| quote!(#ty))
        .unwrap_or_else(|| quote!(()));

    let query_ty = route
        .query_params
        .as_ref()
        .map(|ty| quote!(#ty))
        .unwrap_or_else(|| quote!());

    let query_ty = if route.query_params.is_some() {
        quote!(#query_ty)
    } else {
        quote! { () }
    };

    let body_extractor = generate_body_extractor(&route.request_body);

    let middleware_chain = generate_middleware_chain(route, handler);

    let response_conversion = generate_response_conversion(route.response_body);

    let method = method_tokens(route.method);

    let response_body_type = response_body_ident(route.response_body);

    let endpoint = route.endpoint.value();

    let route_config = generate_route_config(&route.config);

    let registration = generate_route_registration(route.method);

    Ok(quote! {
        pub fn #function(
            router: ::axum::Router<#context>,
            global_config: g_server::Config,
        ) -> ::axum::Router<#context> {
            let mut config =
                global_config;

            #route_config

            #middleware_chain

            let route =
                g_server::route::Route::<_> {
                    method: #method,
                    endpoint: #endpoint,
                    config,
                    response_body_type:
                        g_server::route::ResponseBodyType::#response_body_type,
                    executor,
                };

            #[::rustfmt::skip]
            let route_handler = move |
                ::axum::extract::State(cx):
                    ::axum::extract::State<#context>,

                headers:
                    ::axum::http::HeaderMap,

                ::axum::extract::Path(path_params):
                    ::axum::extract::Path<#path_ty>,

                ::axum::extract::Query(query_params):
                    ::axum::extract::Query<#query_ty>,

                #body_extractor
            | async move {
                let headers =
                    g_server::http::HeaderMap::from(
                        headers
                    );

                let req =
                    g_server::Request {
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
    let mut output = quote! {
        let executor =
            g_server::route::Executor::new(
                #handler
            );
    };

    // Reverse because the first declared middleware
    // must execute first.
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
// Route config
// ============================================================

fn generate_route_config(entries: &[ConfigEntry]) -> TokenStream2 {
    let assignments = entries.iter().map(|entry| {
        let field = &entry.name;
        let value = &entry.value;

        quote! {
            config.#field =
                (#value).into();
        }
    });

    quote! {
        #(#assignments)*
    }
}

// ============================================================
// Axum request body
// ============================================================

fn generate_body_extractor(body: &Option<RequestBody>) -> TokenStream2 {
    match body {
        Some(RequestBody::Json(ty)) => quote! {
            ::axum::extract::Json(body):
                ::axum::extract::Json<#ty>,
        },

        Some(RequestBody::Form(ty)) => quote! {
            ::axum::extract::Form(body):
                ::axum::extract::Form<#ty>,
        },

        None => quote! {
            body: ::axum::body::Body,
        },
    }
}

// ============================================================
// Response conversion
// ============================================================

fn generate_response_conversion(body: ResponseBody) -> TokenStream2 {
    match body {
        ResponseBody::Json => quote! {
            .into_json_response()
        },

        ResponseBody::String => quote! {
            .into_string_response()
        },

        ResponseBody::Html => quote! {
            .into_html_response()
        },
    }
}

fn response_body_ident(body: ResponseBody) -> Ident {
    match body {
        ResponseBody::Json => format_ident!("Json"),

        ResponseBody::String => format_ident!("String"),

        ResponseBody::Html => format_ident!("Html"),
    }
}

// ============================================================
// Axum routing
// ============================================================

fn generate_route_registration(method: HttpMethod) -> TokenStream2 {
    match method {
        HttpMethod::Get => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::get(
                    route_handler
                ),
            )
        },

        HttpMethod::Post => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::post(
                    route_handler
                ),
            )
        },

        HttpMethod::Put => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::put(
                    route_handler
                ),
            )
        },

        HttpMethod::Patch => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::patch(
                    route_handler
                ),
            )
        },

        HttpMethod::Delete => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::delete(
                    route_handler
                ),
            )
        },

        HttpMethod::Options => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::options(
                    route_handler
                ),
            )
        },

        HttpMethod::Head => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::head(
                    route_handler
                ),
            )
        },

        HttpMethod::Trace => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::trace(
                    route_handler
                ),
            )
        },

        // Current DSL semantics:
        // `query` is represented by GET at the Axum layer.
        HttpMethod::Query => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::get(
                    route_handler
                ),
            )
        },

        HttpMethod::Any => quote! {
            router.route(
                route.endpoint,
                ::axum::routing::any(
                    route_handler
                ),
            )
        },
    }
}

// ============================================================
// HttpMethod
// ============================================================

fn method_tokens(method: HttpMethod) -> TokenStream2 {
    let ident = match method {
        HttpMethod::Get => format_ident!("Get"),

        HttpMethod::Post => format_ident!("Post"),

        HttpMethod::Put => format_ident!("Put"),

        HttpMethod::Patch => format_ident!("Patch"),

        HttpMethod::Delete => format_ident!("Delete"),

        HttpMethod::Options => format_ident!("Options"),

        HttpMethod::Head => format_ident!("Head"),

        HttpMethod::Trace => format_ident!("Trace"),

        HttpMethod::Query => format_ident!("Query"),

        HttpMethod::Any => format_ident!("Any"),
    };

    quote! {
        g_server::route::HttpMethod::#ident
    }
}

// ============================================================
// Identifiers
// ============================================================

fn server_ident(server: &Server) -> Ident {
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
