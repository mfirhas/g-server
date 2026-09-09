use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Path, Result};

use crate::{request_body::RequestBody, route::RouteHandler};

pub(crate) fn expand(input: crate::server::GServer) -> Result<TokenStream2> {
    // --------------------------------------------------------
    // SSE / WS / MCP are recognized by the parser, but not
    // implemented yet.
    // --------------------------------------------------------

    for server in &input.servers {
        match server.kind {
            crate::server::ServerKind::Http => {}

            crate::server::ServerKind::Mcp => {
                return Err(syn::Error::new(
                    server.name.span(),
                    "`mcp` server is not implemented yet",
                ));
            }
        }
    }

    let servers = input.servers.iter().collect::<Vec<_>>();

    let main = generate_main(&servers);

    // global infra middlewares configs
    let global_infra_mw = generate_global_infra_middlewares();
    // route infra middlewares configs
    let route_infra_mw = generate_route_infra_middlewares();

    // custom middlewares
    let norm_endpoint_mw = normalize_endpoint_middleware();

    let initializers = servers
        .iter()
        .map(|server| generate_init_function(server))
        .collect::<Result<Vec<_>>>()?;

    let mut routes = Vec::new();

    for server in &servers {
        for (index, route) in server.body.routes.iter().enumerate() {
            routes.push(generate_route_function(server, route, index)?);
        }
    }

    let mut group_functions = Vec::new();

    for server in &servers {
        for group in &server.body.groups {
            generate_group_function(server, group, &[], &mut group_functions)?;
        }
    }

    Ok(quote! {
        use g_server::axum::response::IntoResponse;

        #main

        #global_infra_mw

        #route_infra_mw

        #norm_endpoint_mw

        #(#initializers)*

        #(#routes)*

        #(#group_functions)*
    })
}

// ============================================================
// main()
// ============================================================

fn generate_main(servers: &[&crate::server::Server]) -> TokenStream2 {
    // --------------------------------------------------------
    // OPTIONAL: zero servers.
    //
    // The generated binary is still valid.
    // --------------------------------------------------------

    if servers.is_empty() {
        return quote! {
            #[g_server::tokio::main(crate = "g_server::tokio")]
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
                g_server::tokio::net::TcpListener::bind(
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
            g_server::axum::serve(
                #listener,
                #name.1,
            )
        }
    });

    quote! {
        #[g_server::tokio::main(crate = "g_server::tokio")]
        async fn main() {
            #(#initializers)*

            #(#listeners)*

            g_server::tokio::try_join!(
                #(#serves),*
            )
            .expect(
                "failed running all servers..."
            );
        }
    }
}

fn generate_global_infra_middlewares() -> TokenStream2 {
    quote! {
        fn __register_global_middlewares<C>(
            global_config: &::g_server::Config,
            mut router: g_server::axum::Router<C>,
        ) -> g_server::axum::Router<C>
        where
            C: Clone + Send + Sync + 'static,
        {
            if let Some(c) = global_config.compression {
                router = router.layer(match c {
                    g_server::Compression::All => g_server::tower_http::compression::CompressionLayer::new(),
                    g_server::Compression::Gzip => g_server::tower_http::compression::CompressionLayer::new()
                        .no_br()
                        .no_zstd()
                        .no_deflate(),
                    g_server::Compression::Brotli => g_server::tower_http::compression::CompressionLayer::new()
                        .no_gzip()
                        .no_zstd()
                        .no_deflate(),
                    g_server::Compression::Zstd => g_server::tower_http::compression::CompressionLayer::new()
                        .no_gzip()
                        .no_br()
                        .no_deflate(),
                    g_server::Compression::Deflate => g_server::tower_http::compression::CompressionLayer::new()
                        .no_gzip()
                        .no_br()
                        .no_zstd(),
                });
            }

            if let Some(kib) = global_config.body_limit {
                router = router.layer(g_server::tower_http::limit::RequestBodyLimitLayer::new(kib * 1024));
            }

            if let Some(n) = global_config.concurrency_limit {
                router = router.layer(g_server::tower::limit::ConcurrencyLimitLayer::new(n));
            }

            if let Some(ms) = global_config.timeout {
                router = router.layer(
                    g_server::tower::ServiceBuilder::new()
                        .layer(g_server::axum::error_handling::HandleErrorLayer::new(
                            |err: g_server::tower::BoxError| async move {
                                (g_server::http::StatusCode::REQUEST_TIMEOUT, err.to_string())
                            },
                        ))
                        .layer(g_server::tower::timeout::TimeoutLayer::new(
                            g_server::tokio::time::Duration::from_millis(ms),
                        )),
                );
            }

            if let Some(normalize_endpoint) = global_config.normalize_endpoint && normalize_endpoint {
                router = router.layer(
                    g_server::axum::middleware::from_fn(normalize_endpoint_middleware)
                )
            }

            router
        }
    }
}

fn generate_route_infra_middlewares() -> TokenStream2 {
    quote! {
        fn __register_route_middlewares<C>(
            config: &::g_server::Config,
            mut router: g_server::axum::routing::MethodRouter<C>,
        ) -> g_server::axum::routing::MethodRouter<C>
        where
            C: Clone + Send + Sync + 'static,
        {
            if let Some(c) = config.compression {
                router = router.route_layer(match c {
                    g_server::Compression::All => g_server::tower_http::compression::CompressionLayer::new(),
                    g_server::Compression::Gzip => g_server::tower_http::compression::CompressionLayer::new()
                        .no_br()
                        .no_zstd()
                        .no_deflate(),
                    g_server::Compression::Brotli => g_server::tower_http::compression::CompressionLayer::new()
                        .no_gzip()
                        .no_zstd()
                        .no_deflate(),
                    g_server::Compression::Zstd => g_server::tower_http::compression::CompressionLayer::new()
                        .no_gzip()
                        .no_br()
                        .no_deflate(),
                    g_server::Compression::Deflate => g_server::tower_http::compression::CompressionLayer::new()
                        .no_gzip()
                        .no_br()
                        .no_zstd(),
                });
            }

            if let Some(kib) = config.body_limit {
                router = router.route_layer(g_server::tower_http::limit::RequestBodyLimitLayer::new(kib * 1024));
            }

            if let Some(n) = config.concurrency_limit {
                router = router.route_layer(g_server::tower::limit::ConcurrencyLimitLayer::new(n));
            }

            if let Some(ms) = config.timeout {
                router = router.route_layer(
                    g_server::tower::ServiceBuilder::new()
                        .layer(g_server::axum::error_handling::HandleErrorLayer::new(
                            |err: g_server::tower::BoxError| async move {
                                (g_server::http::StatusCode::REQUEST_TIMEOUT, err.to_string())
                            },
                        ))
                        .layer(g_server::tower::timeout::TimeoutLayer::new(
                            g_server::tokio::time::Duration::from_millis(ms),
                        )),
                );
            }

            if let Some(normalize_endpoint) = config.normalize_endpoint && normalize_endpoint {
                router = router.route_layer(
                    g_server::axum::middleware::from_fn(normalize_endpoint_middleware)
                )
            }

            router
        }
    }
}

// ============================================================
// custom axum middlewares
// ============================================================
fn normalize_endpoint_middleware() -> TokenStream2 {
    quote! {
        pub(crate) async fn normalize_endpoint_middleware(
            request: g_server::axum::extract::Request,
            next: g_server::axum::middleware::Next,
        ) -> g_server::axum::response::Response {
            let path = request.uri().path();

            if path == "/" || (!path.ends_with('/') && !path.contains("//")) {
                return next.run(request).await;
            }

            let query_len = request.uri().query().map_or(0, |q| q.len() + 1);
            let mut normalized = String::with_capacity(path.len() + query_len);
            let mut prev_was_slash = false;

            for ch in path.chars() {
                if ch == '/' {
                    if prev_was_slash {
                        continue;
                    }
                    prev_was_slash = true;
                } else {
                    prev_was_slash = false;
                }
                normalized.push(ch);
            }

            // At most one trailing slash can remain after collapsing, so a single
            // pop is enough — and we never strip the root "/".
            if normalized.len() > 1 && normalized.ends_with('/') {
                normalized.pop();
            }

            if let Some(query) = request.uri().query() {
                normalized.push('?');
                normalized.push_str(query);
            }

            g_server::axum::response::Redirect::permanent(&normalized).into_response()
        }
    }
}

// ============================================================
// __init_<server>()
// ============================================================

fn generate_init_function(server: &crate::server::Server) -> Result<TokenStream2> {
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
        let route = crate::route::route_function_ident(server, index);

        quote! {
            router = #route(router);
        }
    });

    let group_calls = server.body.groups.iter().map(|g| {
        let paths = vec![crate::expr_to_string(&g.prefix).expect("prefix must be a string")];
        let group = group_function_ident(server, &paths);

        quote! {
            router = #group(router);
        }
    });

    Ok(quote! {
        pub fn #init() -> (
            g_server::Server,
            g_server::axum::Router<()>,
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

            let mut router = g_server::axum::Router::<#context_type>::new();

            #(#route_calls)*

            #(#group_calls)*

            router = __register_global_middlewares(&global_config, router);

            let router = router.with_state(context);

            (server, router)
        }
    })
}

// ============================================================
// __route_<server>_<handler>()
// ============================================================

fn generate_route_function(
    server: &crate::server::Server,
    route: &crate::route::Route,
    index: usize,
) -> Result<TokenStream2> {
    let function = crate::route::route_function_ident(server, index);

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
    let body_extractor = crate::axum_impl::generate_body_extractor(&route.request_body);

    let middleware_chain = generate_middleware_chain(route, handler);

    let route_response = generate_route_response(route.response_body);

    let method = route.method.method_tokens();

    let response_body_type = format_ident!("{}", route.response_body.to_string());

    let endpoint = &route.endpoint;

    // Route config starts from inherited global
    // config and overrides only explicitly declared
    // fields.
    let route_config = crate::config::generate_route_config(&route.config);

    let registration = generate_route_registration(route.method);

    Ok(quote! {
        pub fn #function(router: g_server::axum::Router<#context_ty>) -> g_server::axum::Router<#context_ty> {
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
                g_server::axum::extract::State(cx):
                    g_server::axum::extract::State<#context_ty>,

                headers: g_server::axum::http::HeaderMap,

                g_server::axum::extract::Path(path_params):
                    g_server::axum::extract::Path<#path_ty>,

                g_server::axum::extract::Query(query_params):
                    g_server::axum::extract::Query<#query_ty>,

                #body_extractor
            | async move {
                let req = g_server::Request {
                    headers,
                    path_params,
                    query_params,
                    body,
                };

                #route_response
            };

            #registration
        }
    })
}

// ============================================================
// Generate all functions belonging to a root group.
//
// This is the entry point for one root group.
// It recursively walks every nested group and accumulates
// all generated route/group functions into `functions`.
// ============================================================

pub(crate) fn generate_group_function(
    server: &crate::server::Server,
    group: &crate::group::Group,
    middlewares: &[Path],
    functions: &mut Vec<TokenStream2>,
) -> Result<TokenStream2> {
    let prefix = crate::expr_to_string(&group.prefix).ok_or(syn::Error::new(
        Span::call_site(),
        "prefix must be string expression",
    ))?;

    let path = vec![prefix];

    let mut current_middlewares =
        Vec::with_capacity(middlewares.len() + group.middlewares.as_slice().len());
    current_middlewares.extend_from_slice(&middlewares);
    current_middlewares.extend_from_slice(&group.middlewares);

    generate_group_member_function(
        server,
        &crate::group::GroupMember::Group(Box::new(group.clone())),
        &current_middlewares,
        &path,
        functions,
    )
}

// ============================================================
// Generate one group member.
//
// Route:
//     generates the route function and returns the call.
//
// Group:
//     recursively generates the child group and returns
//     the child-group call.
//
// `functions` is shared through the entire recursion.
// ============================================================

fn generate_group_member_function(
    server: &crate::server::Server,
    member: &crate::group::GroupMember,
    middlewares: &[Path],
    path: &[String],
    functions: &mut Vec<TokenStream2>,
) -> Result<TokenStream2> {
    match member {
        // ----------------------------------------------------
        // Route
        // ----------------------------------------------------
        crate::group::GroupMember::Route(route) => {
            let function = group_route_function_ident(server, path, route);

            let mut current_middlewares =
                Vec::with_capacity(middlewares.len() + route.middlewares.as_slice().len());
            current_middlewares.extend_from_slice(&middlewares);
            current_middlewares.extend_from_slice(&route.middlewares);

            let route_function = generate_group_route_function(
                server,
                route,
                &current_middlewares,
                function.clone(),
            )?;

            functions.push(route_function);

            Ok(quote! {
                group_router =
                    #function(group_router);
            })
        }

        // ----------------------------------------------------
        // Nested Group
        // ----------------------------------------------------
        crate::group::GroupMember::Group(group) => {
            let function = group_function_ident(server, path);

            let context_ty = server
                .body
                .context
                .as_ref()
                .map(|ty| quote!(#ty))
                .unwrap_or_else(|| quote!(()));

            let config = crate::config::generate_route_config(&group.config);

            let prefix = &group.prefix;

            let mut member_calls = Vec::new();

            // ------------------------------------------------
            // Recursively process every member.
            // ------------------------------------------------

            for member in &group.members {
                let member_path = match member {
                    crate::group::GroupMember::Route(_) => path.to_vec(),

                    crate::group::GroupMember::Group(child) => {
                        let child_prefix = crate::expr_to_string(&child.prefix).ok_or(
                            syn::Error::new(Span::call_site(), "prefix must be string expression"),
                        )?;

                        let mut child_path = path.to_vec();

                        child_path.push(child_prefix);

                        child_path
                    }
                };

                let mut current_middlewares =
                    Vec::with_capacity(middlewares.len() + group.middlewares.as_slice().len());
                current_middlewares.extend_from_slice(&middlewares);
                current_middlewares.extend_from_slice(&group.middlewares);

                let call = generate_group_member_function(
                    server,
                    member,
                    &current_middlewares,
                    &member_path,
                    functions,
                )?;

                member_calls.push(call);
            }

            // ------------------------------------------------
            // Generate THIS group's function.
            //
            // Notice that this is pushed AFTER recursively
            // generating its children.
            // ------------------------------------------------

            functions.push(quote! {
                pub fn #function(
                    mut router:
                        g_server::axum::Router<#context_ty>,
                ) -> g_server::axum::Router<#context_ty> {
                    #config

                    let mut group_router =
                        g_server::axum::Router::<#context_ty>::new();

                    #(#member_calls)*

                    group_router =
                        __register_global_middlewares(
                            &config,
                            group_router,
                        );

                    router =
                        router.nest(
                            #prefix,
                            group_router,
                        );

                    router
                }
            });

            Ok(quote! {
                group_router =
                    #function(group_router);
            })
        }
    }
}

// group function generation function name
//
// Example: __group_app_name_prefix_1
//
// `prefix_1` is the prefix of group from `path`, sanitized: where slashes replaced with `-`.
fn group_function_ident(server: &crate::server::Server, path: &[String]) -> Ident {
    let mut name = format!("__group_{}", server.name.value());

    for prefix in path {
        name.push('_');
        name.push_str(&crate::group::sanitize_prefix(prefix));
    }

    format_ident!("{}", name)
}

// group function generation generating functions
//
// Example: __route_app_name_prefixes..._handler_name
fn group_route_function_ident(
    server: &crate::server::Server,
    path: &[String],
    route: &crate::route::Route,
) -> Ident {
    let mut name = format!("__route_{}", server.name.value());

    for prefix in path {
        name.push('_');
        name.push_str(&crate::group::sanitize_prefix(prefix));
    }

    let handler_name = match route.handler {
        crate::route::RouteHandler::Path(ref path)
            if path
                .segments
                .last()
                .unwrap()
                .ident
                .to_string()
                .contains("unimplemented_handler") =>
        {
            format!(
                "{}_{}",
                path.segments.last().unwrap().ident.to_string(),
                &crate::random_6_chars()
            )
        }
        crate::route::RouteHandler::Path(ref path) => {
            path.segments.last().unwrap().ident.to_string()
        }
        crate::route::RouteHandler::Closure(_) => crate::random_6_chars(),
    };

    name.push('_');
    name.push_str(&handler_name);

    format_ident!("{}", name)
}

fn generate_group_route_function(
    server: &crate::server::Server,
    route: &crate::route::Route,
    middlewares: &[Path],
    function_ident: Ident,
) -> Result<TokenStream2> {
    let function = function_ident;

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
    let body_extractor = crate::axum_impl::generate_body_extractor(&route.request_body);

    let middleware_chain = generate_group_function_middlewares(middlewares, handler);

    let route_response = generate_route_response(route.response_body);

    let method = route.method.method_tokens();

    let response_body_type = format_ident!("{}", route.response_body.to_string());

    let endpoint = &route.endpoint;

    // Route config starts from inherited global
    // config and overrides only explicitly declared
    // fields.
    let route_config = crate::config::generate_route_config(&route.config);

    let registration = generate_route_registration(route.method);

    Ok(quote! {
        pub fn #function(router: g_server::axum::Router<#context_ty>) -> g_server::axum::Router<#context_ty> {
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
                g_server::axum::extract::State(cx):
                    g_server::axum::extract::State<#context_ty>,

                headers: g_server::axum::http::HeaderMap,

                g_server::axum::extract::Path(path_params):
                    g_server::axum::extract::Path<#path_ty>,

                g_server::axum::extract::Query(query_params):
                    g_server::axum::extract::Query<#query_ty>,

                #body_extractor
            | async move {
                let req = g_server::Request {
                    headers,
                    path_params,
                    query_params,
                    body,
                };

                #route_response
            };

            #registration
        }
    })
}

fn generate_group_function_middlewares(
    middlewares: &[Path],
    handler: &RouteHandler,
) -> TokenStream2 {
    let mut output = match handler {
        RouteHandler::Path(handler) => quote! {
            let executor =
                g_server::route::Executor::new(
                    #handler
                );
        },
        RouteHandler::Closure(closure) => quote! {
            let executor =
                g_server::route::Executor::new(
                    async move #closure
                );
        },
    };

    for middleware in middlewares.iter().rev() {
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

pub(crate) fn generate_body_extractor(body: &Option<RequestBody>) -> TokenStream2 {
    match body {
        // JSON body:
        //
        // request_body: Json(MyStruct)
        Some(RequestBody::Json(ty)) => {
            quote! {
                g_server::axum::extract::Json(body):
                    g_server::axum::extract::Json<#ty>,
            }
        }

        // Form body:
        //
        // request_body: Form(MyStruct)
        Some(RequestBody::Form(ty)) => {
            quote! {
                g_server::axum::extract::Form(body):
                    g_server::axum::extract::Form<#ty>,
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
// Middleware chain
// ============================================================

fn generate_middleware_chain(route: &crate::route::Route, handler: &RouteHandler) -> TokenStream2 {
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

    let mut output = match handler {
        RouteHandler::Path(handler) => quote! {
            let executor =
                g_server::route::Executor::new(
                    #handler
                );
        },
        RouteHandler::Closure(closure) => quote! {
            let executor =
                g_server::route::Executor::new(
                    async move #closure
                );
        },
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
// Response conversion
// ============================================================

fn generate_route_response(body: crate::response_body::ResponseBody) -> TokenStream2 {
    let resp_body_type = match body {
        crate::response_body::ResponseBody::Json => {
            quote! {
                into_axum_json()
            }
        }

        crate::response_body::ResponseBody::String => {
            quote! {
                into_axum_string()
            }
        }

        crate::response_body::ResponseBody::Html => {
            quote! {
                into_axum_html()
            }
        }

        crate::response_body::ResponseBody::Empty => {
            quote! {
                into_axum_empty()
            }
        }
    };

    quote! {
        match route.executor.exec(cx, req).await {
            Ok(resp) => resp.#resp_body_type,
            Err(err) => err.#resp_body_type,
        }
    }
}

// ============================================================
// Axum routing
// ============================================================

fn generate_route_registration(method: crate::server::HttpMethod) -> TokenStream2 {
    match method {
        crate::server::HttpMethod::Get => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::get(route_handler)))
            }
        }

        crate::server::HttpMethod::Post => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::post(route_handler)))
            }
        }

        crate::server::HttpMethod::Put => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::put(route_handler)))
            }
        }

        crate::server::HttpMethod::Patch => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::patch(route_handler)))
            }
        }

        crate::server::HttpMethod::Delete => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::delete(route_handler)))
            }
        }

        crate::server::HttpMethod::Options => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::options(route_handler)))
            }
        }

        crate::server::HttpMethod::Head => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::head(route_handler)))
            }
        }

        crate::server::HttpMethod::Trace => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::trace(route_handler)))
            }
        }

        // Current DSL semantics:
        // `query` is represented by GET at the Axum layer.
        crate::server::HttpMethod::Query => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::get(route_handler)))
            }
        }

        crate::server::HttpMethod::Any => {
            quote! {
                router.route(route.endpoint, __register_route_middlewares(&route.config, g_server::axum::routing::any(route_handler)))
            }
        }
    }
}

// ============================================================
// Identifiers
// ============================================================

fn server_ident(server: &crate::server::Server) -> Ident {
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

fn init_ident(server: &crate::server::Server) -> Ident {
    format_ident!("__init_{}", server.name.value())
}
