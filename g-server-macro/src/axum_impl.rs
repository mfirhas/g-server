use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Expr, ExprLit, Lit, Path, Result};

use crate::{request_body::RequestBody, route::RouteHandler, server::HttpMethod};

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
        use g_server::BadRequestErrorMessage;

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
        let server_name = &server.name;

        let name = server_ident(server);

        let init = init_ident(server);

        quote! {
            let #name = match #init().await {
                Ok(ctx) => ctx,
                Err(err) => {
                    eprintln!("g-server: failed initializing context of `{}`: {}", #server_name, err);
                    return;
                },
            };
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

            if let Some(bytes) = global_config.body_limit {
                router = router.layer(g_server::axum::extract::DefaultBodyLimit::max(bytes));
            }

            match (global_config.concurrency_limit, global_config.concurrency_limit_error) {
                (Some(n), Some(err_handler)) => {
                    router = router.layer(
                        g_server::tower::ServiceBuilder::new()
                            .layer(g_server::axum::error_handling::HandleErrorLayer::new(
                                move |err: g_server::tower::BoxError| async move {
                                    err_handler()
                                },
                            ))
                            .layer(g_server::tower::load_shed::LoadShedLayer::new())
                            .layer(g_server::tower::limit::ConcurrencyLimitLayer::new(n)),
                    );
                }
                (Some(n), _) => {
                    router = router.layer(g_server::tower::limit::ConcurrencyLimitLayer::new(n));
                }
                (_, Some(_)) => {},
                _ => {}
            }

            match (global_config.timeout, global_config.timeout_error) {
                (Some(ms), Some(err_handler)) => {
                    router = router.layer(
                        g_server::tower::ServiceBuilder::new()
                            .layer(g_server::axum::error_handling::HandleErrorLayer::new(
                                move |err: g_server::tower::BoxError| async move {
                                    err_handler()
                                },
                            ))
                            .layer(g_server::tower::timeout::TimeoutLayer::new(
                                g_server::tokio::time::Duration::from_millis(ms),
                            )),
                    );
                },
                (Some(ms), _) => {
                    router = router.layer(
                        g_server::tower::ServiceBuilder::new()
                            .layer(g_server::axum::error_handling::HandleErrorLayer::new(
                                move |err: g_server::tower::BoxError| async move {
                                    (g_server::http::StatusCode::GATEWAY_TIMEOUT, err.to_string()).into_response()
                                },
                            ))
                            .layer(g_server::tower::timeout::TimeoutLayer::new(
                                g_server::tokio::time::Duration::from_millis(ms),
                            )),
                    );
                },
                (_, Some(_)) => {},
                _ => {}
            }

            if let Some(ref cors) = global_config.cors {
                router = router.layer(g_server::config::Cors::layer(cors.clone()))
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

            if let Some(bytes) = config.body_limit {
                router = router.route_layer(g_server::axum::extract::DefaultBodyLimit::max(bytes));
            }

            match (config.concurrency_limit, config.concurrency_limit_error) {
                (Some(n), Some(err_handler)) => {
                    router = router.route_layer(
                        g_server::tower::ServiceBuilder::new()
                            .layer(g_server::axum::error_handling::HandleErrorLayer::new(
                                move |err: g_server::tower::BoxError| async move {
                                    err_handler()
                                },
                            ))
                            .layer(g_server::tower::load_shed::LoadShedLayer::new())
                            .layer(g_server::tower::limit::ConcurrencyLimitLayer::new(n)),
                    );
                }
                (Some(n), _) => {
                    router = router.route_layer(g_server::tower::limit::ConcurrencyLimitLayer::new(n));
                }
                (_, Some(_)) => {},
                _ => {}
            }

            match (config.timeout, config.timeout_error) {
                (Some(ms), Some(err_handler)) => {
                    router = router.route_layer(
                        g_server::tower::ServiceBuilder::new()
                            .layer(g_server::axum::error_handling::HandleErrorLayer::new(
                                move |err: g_server::tower::BoxError| async move {
                                    err_handler()
                                },
                            ))
                            .layer(g_server::tower::timeout::TimeoutLayer::new(
                                g_server::tokio::time::Duration::from_millis(ms),
                            )),
                    );
                },
                (Some(ms), _) => {
                    router = router.route_layer(
                        g_server::tower::ServiceBuilder::new()
                            .layer(g_server::axum::error_handling::HandleErrorLayer::new(
                                move |err: g_server::tower::BoxError| async move {
                                    (g_server::http::StatusCode::GATEWAY_TIMEOUT, err.to_string()).into_response()
                                },
                            ))
                            .layer(g_server::tower::timeout::TimeoutLayer::new(
                                g_server::tokio::time::Duration::from_millis(ms),
                            )),
                    );
                },
                (_, Some(_)) => {},
                _ => {}
            }

            if let Some(ref cors) = config.cors {
                router = router.route_layer(g_server::config::Cors::layer(cors.clone()))
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
            let context = <#ty>::init().await.map_err(|err| err.to_string())?;
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
        pub async fn #init() -> std::result::Result<(g_server::Server, g_server::axum::Router<()>), String> {
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

            Ok((server, router))
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

    // Route config starts from inherited global
    // config and overrides only explicitly declared
    // fields.
    let route_config = crate::config::generate_route_config(&route.config);

    let registration = generate_route_registration(route.method, &route.endpoint);

    if route.method == HttpMethod::File {
        let embed_path = route
            .config
            .iter()
            .find(|cfg| cfg.name.to_string() == crate::config::CONFIG_FIELD_FILE_DIR)
            .map(|cfg| cfg.value.clone())
            .unwrap_or(syn::parse_quote!(""));
        let endpoint = if route.config.iter().any(|cfg| {
            cfg.name.to_string() == crate::config::CONFIG_FIELD_FILE_EMBED
                && matches!(
                    &cfg.value,
                    Expr::Lit(ExprLit {
                        lit: Lit::Bool(lit),
                        ..
                    }) if lit.value
                )
        }) {
            crate::append_literal(&route.endpoint, "/{*path}")?
        } else {
            route.endpoint.clone()
        };
        return Ok(generate_file_handler_registration(
            route,
            function,
            context_ty,
            route_config,
            &endpoint,
            registration,
            &embed_path,
        ));
    }

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

    let handler_chain = generate_handler_chain(&route.middlewares, handler);

    let route_response = generate_route_response(&route.response_body);

    let input_extractor = generate_input_extractor(route, path_ty, query_ty, &route.request_body);

    let bad_request_handler = generate_bad_request_error_handler(route, &route.request_body);

    let form_data_parsing = generate_form_data_parsing(&route.request_body);

    let handler_registration = generate_handler_registration(
        function,
        context_ty,
        route_config,
        handler_chain,
        input_extractor,
        bad_request_handler,
        form_data_parsing,
        route_response,
        registration,
    );

    Ok(handler_registration)
}

fn generate_handler_registration(
    func: Ident,
    context_ty: TokenStream2,
    route_config: TokenStream2,
    handler_chain: TokenStream2,
    input_extractor: TokenStream2,
    bad_request_handler: TokenStream2,
    form_data_parsing: TokenStream2,
    route_response: TokenStream2,
    registration: TokenStream2,
) -> TokenStream2 {
    quote! {
        pub fn #func(router: g_server::axum::Router<#context_ty>) -> g_server::axum::Router<#context_ty> {
            // Then override route-specific fields.
            #route_config

            // Handler + optional middleware chain.
            #handler_chain

            let route_handler = move |
                method: g_server::http::Method,
                g_server::axum::extract::State(cx):
                    g_server::axum::extract::State<#context_ty>,

                headers: g_server::axum::http::HeaderMap,

                #input_extractor
            | async move {
                #bad_request_handler

                #form_data_parsing

                let req = g_server::Request {
                    method: method.into(),
                    headers,
                    path_params,
                    query_params,
                    body,
                };

                #route_response
            };

            #registration
        }
    }
}

fn generate_file_handler_registration(
    route: &crate::route::Route,
    func: Ident,
    context_ty: TokenStream2,
    route_config: TokenStream2,
    endpoint: &Expr,
    registration: TokenStream2,
    embed_path: &Expr,
) -> TokenStream2 {
    let orig_endpoint = &route.endpoint;
    quote! {
        pub fn #func(router: g_server::axum::Router<#context_ty>) -> g_server::axum::Router<#context_ty> {
            #route_config

            use g_server::tower_http::services::{ServeDir, ServeFile};

            let file_server_route = if let Some(is_embed) = config.embed && is_embed {
                #[cfg(feature = "embed")]
                {
                    #[derive(g_server::rust_embed::RustEmbed)]
                    #[folder = #embed_path]
                    struct EmbedFS;

                    let serve_embedded = move |uri: g_server::axum::http::Uri| async move {
                        let path = uri.path()
                            .strip_prefix(#orig_endpoint)
                            .unwrap_or(uri.path())
                            .trim_start_matches('/');

                        match EmbedFS::get(path) {
                            Some(file) => {
                                let body = g_server::axum::body::Body::from(file.data.into_owned());

                                let ret = g_server::axum::response::Response::builder()
                                    .header(
                                        g_server::axum::http::header::CONTENT_TYPE,
                                        g_server::mime_guess::from_path(path)
                                            .first_or_octet_stream()
                                            .as_ref(),
                                    )
                                    .body(body);

                                match ret {
                                    Ok(res) => res,
                                    Err(err) => {
                                        if let Some(ref not_found_file) = config.fallback_file {
                                            if let Some(not_found) = EmbedFS::get(not_found_file) {
                                                let body = g_server::axum::body::Body::from(not_found.data.into_owned());
                                                let ret = g_server::axum::response::Response::builder()
                                                    .header(
                                                        g_server::axum::http::header::CONTENT_TYPE,
                                                        g_server::mime_guess::from_path(path)
                                                            .first_or_octet_stream()
                                                            .as_ref(),
                                                    )
                                                    .body(body);
                                                match ret {
                                                    Ok(res) => res,
                                                    Err(err) => g_server::Response::new().with_status(g_server::StatusCode::NOT_FOUND).with_text(format!("g-server: failed building fallback response: {}", err)).into_axum_string(),
                                                }
                                            } else {
                                                g_server::Response::new().with_status(g_server::StatusCode::NOT_FOUND).with_text("g-server: fallback file not found").into_axum_string()
                                            }
                                        } else {
                                            g_server::Response::new().with_status(g_server::StatusCode::NOT_FOUND).with_text("g-server: failed building response").into_axum_string()
                                        }
                                    },
                                }
                            }
                            None => {
                                if let Some(ref not_found_file) = config.fallback_file {
                                    if let Some(not_found) = EmbedFS::get(not_found_file) {
                                        let body = g_server::axum::body::Body::from(not_found.data.into_owned());
                                        let ret = g_server::axum::response::Response::builder()
                                            .header(
                                                g_server::axum::http::header::CONTENT_TYPE,
                                                g_server::mime_guess::from_path(path)
                                                    .first_or_octet_stream()
                                                    .as_ref(),
                                            )
                                            .body(body);
                                        match ret {
                                            Ok(res) => res,
                                            Err(err) => g_server::Response::new().with_status(g_server::StatusCode::NOT_FOUND).with_text(format!("g-server: failed building fallback's fallback response: {}", err)).into_axum_string(),
                                        }
                                    } else {
                                        g_server::Response::new().with_status(g_server::StatusCode::NOT_FOUND).with_text("g-server: fallback file's fallback file not found").into_axum_string()
                                    }
                                } else {
                                    g_server::Response::new().with_status(g_server::StatusCode::NOT_FOUND).with_text("g-server: file not found").into_axum_string()
                                }
                            },
                        }
                    };

                    g_server::axum::Router::new().route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::get(serve_embedded)))
                }

                #[cfg(not(feature = "embed"))]
                {
                    panic!("g-server: you shall not pass...!!")
                }
            } else {
                if let Some(ref not_found_file) = config.fallback_file {
                    g_server::axum::Router::new().nest_service(#endpoint, ServeDir::new(config.dir.unwrap_or_default()).not_found_service(ServeFile::new(not_found_file)))
                } else {
                    g_server::axum::Router::new().nest_service(#endpoint, ServeDir::new(config.dir.unwrap_or_default()))
                }
            };

            #registration
        }
    }
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
    let mut name = format!("__route_{}_{}", server.name.value(), route.method);

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
            format!(
                "{}_{}",
                path.segments.last().unwrap().ident,
                crate::random_6_chars()
            )
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

    // Route config starts from inherited global
    // config and overrides only explicitly declared
    // fields.
    let route_config = crate::config::generate_route_config(&route.config);

    let registration = generate_route_registration(route.method, &route.endpoint);

    if route.method == HttpMethod::File {
        let embed_path = route
            .config
            .iter()
            .find(|cfg| cfg.name.to_string() == crate::config::CONFIG_FIELD_FILE_DIR)
            .map(|cfg| cfg.value.clone())
            .unwrap_or(syn::parse_quote!(""));
        let endpoint = if route.config.iter().any(|cfg| {
            cfg.name.to_string() == crate::config::CONFIG_FIELD_FILE_EMBED
                && matches!(
                    &cfg.value,
                    Expr::Lit(ExprLit {
                        lit: Lit::Bool(lit),
                        ..
                    }) if lit.value
                )
        }) {
            crate::append_literal(&route.endpoint, "/{*path}")?
        } else {
            route.endpoint.clone()
        };
        return Ok(generate_file_handler_registration(
            route,
            function,
            context_ty,
            route_config,
            &endpoint,
            registration,
            &embed_path,
        ));
    }

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

    let handler_chain = generate_handler_chain(middlewares, handler);

    let route_response = generate_route_response(&route.response_body);

    let input_extractor = generate_input_extractor(route, path_ty, query_ty, &route.request_body);

    let bad_request_handler = generate_bad_request_error_handler(route, &route.request_body);

    let form_data_parsing = generate_form_data_parsing(&route.request_body);

    let handler_registration = generate_handler_registration(
        function,
        context_ty,
        route_config,
        handler_chain,
        input_extractor,
        bad_request_handler,
        form_data_parsing,
        route_response,
        registration,
    );

    Ok(handler_registration)
}

fn generate_input_extractor(
    route: &crate::route::Route,
    path_ty: TokenStream2,
    query_ty: TokenStream2,
    body: &Option<RequestBody>,
) -> TokenStream2 {
    let path_query_token = if route
        .config
        .iter()
        .any(|cfg| cfg.name.to_string() == crate::config::CONFIG_FIELD_BAD_REQUEST_ERROR)
    {
        quote! {
            path_params: std::result::Result<g_server::axum::extract::Path<#path_ty>, g_server::axum::extract::rejection::PathRejection>,
            query_params: std::result::Result<g_server::axum::extract::Query<#query_ty>, g_server::axum::extract::rejection::QueryRejection>,
        }
    } else {
        quote! {
            g_server::axum::extract::Path(path_params): g_server::axum::extract::Path<#path_ty>,
            g_server::axum::extract::Query(query_params): g_server::axum::extract::Query<#query_ty>,
        }
    };

    let input = if route
        .config
        .iter()
        .any(|cfg| cfg.name.to_string() == crate::config::CONFIG_FIELD_BAD_REQUEST_ERROR)
    {
        match body {
            // JSON body:
            //
            // request_body: Json(MyStruct)
            Some(RequestBody::Json(ty)) => {
                quote! {
                    #path_query_token
                    body: std::result::Result<g_server::axum::extract::Json<#ty>, g_server::axum::extract::rejection::JsonRejection>,
                }
            }

            // Form body:
            //
            // request_body: Form(MyStruct)
            Some(RequestBody::Form(ty)) => {
                quote! {
                    #path_query_token
                    body: std::result::Result<g_server::axum::extract::Form<#ty>, g_server::axum::extract::rejection::FormRejection>,
                }
            }

            Some(RequestBody::FormData(_)) => {
                quote! {
                    #path_query_token
                    multipart: std::result::Result<g_server::axum::extract::Multipart, g_server::axum::extract::multipart::MultipartRejection>,
                }
            }

            // String body:
            //
            // request_body: String
            Some(RequestBody::String) => {
                quote! {
                    #path_query_token
                    body: std::result::Result<String, g_server::axum::extract::rejection::StringRejection>,
                }
            }

            // No request_body:
            //
            // body is simply ().
            None => {
                quote! {
                    #path_query_token
                    body: (),
                }
            }
        }
    } else {
        match body {
            // JSON body:
            //
            // request_body: Json(MyStruct)
            Some(RequestBody::Json(ty)) => {
                quote! {
                    #path_query_token
                    g_server::axum::extract::Json(body):
                        g_server::axum::extract::Json<#ty>,
                }
            }

            // Form body:
            //
            // request_body: Form(MyStruct)
            Some(RequestBody::Form(ty)) => {
                quote! {
                    #path_query_token
                    g_server::axum::extract::Form(body):
                        g_server::axum::extract::Form<#ty>,
                }
            }

            Some(RequestBody::FormData(_)) => {
                quote! {
                    #path_query_token
                    mut multipart: g_server::axum::extract::Multipart,
                }
            }

            // String body:
            //
            // request_body: String
            Some(RequestBody::String) => {
                quote! {
                    #path_query_token
                    body: String,
                }
            }

            // No request_body:
            //
            // body is simply ().
            None => {
                quote! {
                    #path_query_token
                    body: (),
                }
            }
        }
    };

    input
}

fn generate_bad_request_error_handler(
    route: &crate::route::Route,
    body: &Option<RequestBody>,
) -> TokenStream2 {
    if route
        .config
        .iter()
        .any(|cfg| cfg.name.to_string() == crate::config::CONFIG_FIELD_BAD_REQUEST_ERROR)
    {
        let body_bad_request_error_handler = match body {
            // JSON body:
            //
            // request_body: Json(MyStruct)
            Some(RequestBody::Json(_)) => {
                quote! {
                    let body = match body {
                        Ok(g_server::axum::extract::Json(body)) => body,
                        Err(err) => return (config.bad_request_error.unwrap_or(
                            |_| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text("g-server: bad request, sir!").into_axum_string()
                        ))(err.body_text().as_str()),
                    };
                }
            }

            // Form body:
            //
            // request_body: Form(MyStruct)
            Some(RequestBody::Form(_)) => {
                quote! {
                    let body = match body {
                        Ok(g_server::axum::extract::Form(body)) => body,
                        Err(err) => return (config.bad_request_error.unwrap_or(
                            |_| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text("g-server: bad request, sir!").into_axum_string()
                        ))(err.body_text().as_str()),
                    };
                }
            }

            Some(RequestBody::FormData(_)) => {
                quote! {
                    let mut multipart = match multipart {
                        Ok(body) => body,
                        Err(err) => return (config.bad_request_error.unwrap_or(
                            |_| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text("g-server: bad request, sir!").into_axum_string()
                        ))(err.body_text().as_str()),
                    };
                }
            }

            // String body:
            //
            // request_body: String
            Some(RequestBody::String) => {
                quote! {
                    let body = match body {
                        Ok(body) => body,
                        Err(err) => return (config.bad_request_error.unwrap_or(
                            |_| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text("g-server: bad request, sir!").into_axum_string()
                        ))(err.body_text().as_str()),
                    };
                }
            }

            // No request_body:
            //
            // body is simply ().
            None => {
                quote! {}
            }
        };

        return quote! {
            let path_params = match path_params {
                Ok(g_server::axum::extract::Path(path_params)) => path_params,
                Err(err) => return (config.bad_request_error.unwrap_or(
                    |_| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text("g-server: bad request, sir!").into_axum_string()
                ))(err.body_text().as_str()),
            };

            let query_params = match query_params {
                Ok(g_server::axum::extract::Query(query_params)) => query_params,
                Err(err) => return (config.bad_request_error.unwrap_or(
                    |_| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text("g-server: bad request, sir!").into_axum_string()
                ))(err.body_text().as_str()),
            };

            #body_bad_request_error_handler
        };
    }

    quote! {}
}

fn generate_form_data_parsing(body: &Option<RequestBody>) -> TokenStream2 {
    if let Some(formdata) = body
        && let RequestBody::FormData(ty) = formdata
    {
        return quote! {
            let mut body: g_server::multipart::FormData<#ty> = g_server::multipart::FormData::empty();
            let mut fields = Vec::<(String, String)>::new();
            let mut files = Vec::<g_server::multipart::Data>::new();
            while let Some(field) = match multipart.next_field().await {
                Ok(field) => field,
                Err(err) => return (config.bad_request_error.unwrap_or(
                    |err| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text(format!("g-server: error while reading form-data fields: {}", err)).into_axum_string()
                ))(err.to_string().as_str())
            } {
                let name = match field.name() {
                    Some(name) => name.to_owned(),
                    None => return (config.bad_request_error.unwrap_or(
                                |err| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text(format!("{}", err)).into_axum_string()
                            ))("g-server: multipart form-data field name is required")
                };

                let filename = field.file_name().map(str::to_owned);
                let content_type = field.content_type().map(str::to_owned);

                if let Some(filename) = filename {
                    let bytes = match field.bytes().await {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            return (config.bad_request_error.unwrap_or(
                                |err| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text(format!("g-server: filename exists, but failed reading file: {}", err)).into_axum_string()
                            ))(err.to_string().as_str())
                        }
                    };

                    files.push(g_server::multipart::Data {
                        name,
                        filename: Some(filename),
                        content_type,
                        file: bytes,
                    });
                } else {
                    let value = match field.text().await {
                        Ok(value) => value,
                        Err(err) => {
                            return (config.bad_request_error.unwrap_or(
                                |err| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text(format!("g-server: failed reading form-data text fields: {}", err)).into_axum_string()
                            ))(err.to_string().as_str())
                        }
                    };

                    fields.push((name, value));
                }
            }

            let form = if !fields.is_empty() {
                let encoded = match g_server::serde_urlencoded::to_string(&fields) {
                    Ok(encoded) => encoded,
                    Err(err) => return (config.bad_request_error.unwrap_or(
                                        |err| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text(format!("g-server: failed encoding form-data text fields: {}", err)).into_axum_string()
                                    ))(err.to_string().as_str())
                };

                let form = match g_server::serde_urlencoded::from_str::<#ty>(&encoded) {
                    Ok(form) => form,
                    Err(err) => return (config.bad_request_error.unwrap_or(
                                        |err| g_server::Response::new().with_status(g_server::StatusCode::BAD_REQUEST).with_text(format!("g-server: failed parsing encoded form-data text fields: {}", err)).into_axum_string()
                                    ))(err.to_string().as_str())
                };

                Some(form)
            } else {
                None
            };

            body.form = form;
            if !files.is_empty() {
                body.data = Some(files);
            }
        };
    }

    quote! {}
}

// ============================================================
// Handler chain
// ============================================================

fn generate_handler_chain(middlewares: &[Path], handler: &RouteHandler) -> TokenStream2 {
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

// ============================================================
// Response conversion
// ============================================================

fn generate_route_response(body: &crate::response_body::ResponseBody) -> TokenStream2 {
    match body {
        crate::response_body::ResponseBody::Json(ty) => {
            if matches!(ty, syn::Type::Tuple(t) if t.elems.is_empty()) {
                quote! {
                    let res: g_server::Result<_, _> = executor.exec(cx, req).await;
                    match res {
                        Ok(resp) => resp.into_axum_json(),
                        Err(err) => err.into_axum_json(),
                    }
                }
            } else {
                quote! {
                    let res: g_server::Result<#ty, _> = executor.exec(cx, req).await;
                    match res {
                        Ok(resp) => resp.into_axum_json(),
                        Err(err) => err.into_axum_json(),
                    }
                }
            }
        }

        crate::response_body::ResponseBody::String => {
            quote! {
                let res: g_server::Result<_, _> = executor.exec(cx, req).await;
                match res {
                    Ok(resp) => resp.into_axum_string(),
                    Err(err) => err.into_axum_string(),
                }
            }
        }

        crate::response_body::ResponseBody::Html => {
            quote! {
                let res: g_server::Result<_, _> = executor.exec(cx, req).await;
                match res {
                    Ok(resp) => resp.into_axum_html(),
                    Err(err) => err.into_axum_html(),
                }
            }
        }

        crate::response_body::ResponseBody::Empty => {
            quote! {
                let res: g_server::Result<(), _> = executor.exec(cx, req).await;
                match res {
                    Ok(resp) => resp.into_axum_empty(),
                    Err(err) => err.into_axum_empty(),
                }
            }
        }
    }
}

// ============================================================
// Axum routing
// ============================================================

fn generate_route_registration(method: crate::server::HttpMethod, endpoint: &Expr) -> TokenStream2 {
    match method {
        crate::server::HttpMethod::Get => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::get(route_handler)))
            }
        }

        crate::server::HttpMethod::Post => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::post(route_handler)))
            }
        }

        crate::server::HttpMethod::Put => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::put(route_handler)))
            }
        }

        crate::server::HttpMethod::Patch => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::patch(route_handler)))
            }
        }

        crate::server::HttpMethod::Delete => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::delete(route_handler)))
            }
        }

        crate::server::HttpMethod::Options => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::options(route_handler)))
            }
        }

        crate::server::HttpMethod::Head => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::head(route_handler)))
            }
        }

        crate::server::HttpMethod::Trace => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::trace(route_handler)))
            }
        }

        // Current DSL semantics:
        // `query` is represented by GET at the Axum layer.
        crate::server::HttpMethod::Query => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::get(route_handler)))
            }
        }

        crate::server::HttpMethod::Any => {
            quote! {
                router.route(#endpoint, __register_route_middlewares(&config, g_server::axum::routing::any(route_handler)))
            }
        }

        crate::server::HttpMethod::File => {
            quote! {
                if let Some(is_embed) = config.embed && is_embed {
                    router.merge(file_server_route)
                } else {
                    router.merge(__register_global_middlewares(&config, file_server_route))
                }
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
