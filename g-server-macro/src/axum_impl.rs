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

    let servers = input
        .servers
        .iter()
        .filter(|s| matches!(&s.kind, crate::server::ServerKind::Http))
        .collect::<Vec<_>>();

    let main = generate_main(&servers);

    // global infra middlewares configs
    let global_infra_mw = generate_global_infra_middlewares();
    // route infra middlewares configs
    let route_infra_mw = generate_route_infra_middlewares();

    // custom middlewares
    let norm_endpoint_mw = normalize_endpoint_middleware();
    let request_id_header_mw = request_id_header_middleware();
    let tracing_mw = tracing_middleware();

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
        #request_id_header_mw
        #tracing_mw

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
                    eprintln!("g-server: failed initializing server `{}`: {}", #server_name, err);
                    return;
                },
            };
        }
    });

    let listeners = servers.iter().map(|server| {
        if server
            .body
            .config
            .iter()
            .any(|cfg| cfg.name.to_string() == crate::config::CONFIG_FIELD_TLS)
        {
            return quote! {};
        }

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

    let mut is_graceful = false;
    // graceful shutdown cancellation token
    let grace_shutdown_canc_token: TokenStream2 = if servers.iter().any(|s| {
        s.body.config.iter().any(|cfg| {
            cfg.name.to_string() == crate::config::CONFIG_FIELD_GRACEFUL_SHUTDOWN
                && crate::extract_bool(&cfg.value)
        })
    }) {
        is_graceful = true;

        let tls_grace_handle = if servers.iter().any(|srv| {
            srv.body
                .config
                .iter()
                .any(|cfg| cfg.name.to_string() == crate::config::CONFIG_FIELD_TLS)
        }) {
            (
                quote! { let handle = g_server::axum_server::Handle::new(); },
                quote! { let handle = handle.clone(); },
                quote! { handle.graceful_shutdown(Some(std::time::Duration::from_secs(5))); },
            )
        } else {
            (quote! {}, quote! {}, quote! {})
        };
        let tls_grace_handle_init = tls_grace_handle.0;
        let tls_grace_handle_clone = tls_grace_handle.1;
        let tls_grace_handle_shutdown = tls_grace_handle.2;

        quote! {
            let shutdown_signal = async || {
                let ctrl_c = async {
                    g_server::tokio::signal::ctrl_c()
                        .await
                        .expect("failed to install Ctrl+C handler");
                };

                #[cfg(unix)]
                let terminate = async {
                    g_server::tokio::signal::unix::signal(
                        g_server::tokio::signal::unix::SignalKind::terminate(),
                    )
                    .expect("failed to install SIGTERM handler")
                    .recv()
                    .await;
                };

                #[cfg(not(unix))]
                let terminate = std::future::pending::<()>();

                g_server::tokio::select! {
                    _ = ctrl_c => {},
                    _ = terminate => {},
                }
            };

            let shutdown = g_server::tokio_util::sync::CancellationToken::new();
            #tls_grace_handle_init
            {
                let signal_shutdown = shutdown.clone();
                #tls_grace_handle_clone
                g_server::tokio::spawn(async move {
                    shutdown_signal().await;
                    println!("shutdown signal received...");
                    signal_shutdown.cancel();
                    println!("shutting down...");
                    #tls_grace_handle_shutdown
                });
            }
        }
    } else {
        quote! {}
    };

    let mut tls_config: Vec<TokenStream2> = vec![];
    #[cfg(feature = "tls")]
    {
        servers.iter().for_each(|server| {
            if let Some(tls_cfg) = server
                .body
                .config
                .iter()
                .find(|cfg| cfg.name.to_string() == crate::config::CONFIG_FIELD_TLS)
            {
                let name = server_ident(server);
                let tls_config_name = format_ident!("{}_tls_config", name);
                let tls_config_value = &tls_cfg.value;
                let config_factory = quote! {
                    let #tls_config_name = if (#tls_config_value).client_cas.is_some() {
                        (#tls_config_value).mtls_config()
                            .expect(format!("g-server: failed reading and creating mtls config for {}", #name.0.name).as_str())
                    } else {
                        (#tls_config_value).tls_config().await
                            .expect(format!("g-server: failed reading and creating tls config for {}", #name.0.name).as_str())
                    };
                };
                let host = server.ip.value();
                let https_port: u16 = server.port.base10_parse().expect("invalid port, expect u16");
                let redirect_http_to_https = quote! {
                    if (#tls_config_value).client_cas.is_none() && let Some(http_port) = (#tls_config_value).redirect_from_port {
                        g_server::tokio::spawn(g_server::config::Tls::redirect_http_to_https(#host, http_port, #https_port));
                    }
                };
                tls_config.push(config_factory);
                tls_config.push(redirect_http_to_https);
                tls_config.push(quote! {
                    let running_msg = if (#tls_config_value).client_cas.is_some() {
                        "mTLS"
                    } else {
                        "TLS"
                    };
                    println!(
                        "g-server({}): running {} on {}:{}...",
                        running_msg,
                        #name.0.name,
                        #name.0.ip_address,
                        #name.0.port
                    );
                });
            }
        });
    }

    let serves = servers.iter().map(|server| {
        let name = server_ident(server);

        let listener = format_ident!("{}_listener", name);

        #[cfg(feature = "tls")]
        {
            if let Some(_) = server
                .body
                .config
                .iter()
                .find(|cfg| cfg.name.to_string() == crate::config::CONFIG_FIELD_TLS)
            {
                return generate_tls_server(&name, is_graceful);
            }
        }

        if is_graceful {
            return quote! {
                g_server::axum::serve(
                    #listener,
                    #name.1.into_make_service_with_connect_info::<std::net::SocketAddr>(),
                ).with_graceful_shutdown(shutdown.clone().cancelled_owned())
            };
        }

        quote! {
            g_server::axum::serve(
                #listener,
                #name.1.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
        }
    });

    let logging_init = servers
        .iter()
        .find_map(|server| {
            server
                .body
                .config
                .iter()
                .find(|config| config.name == crate::config::CONFIG_FIELD_LOGGING)
        })
        .map(|logging_config| {
            let logging_config = &logging_config.value;

            let trace_log = servers.iter().find_map(|server| {
                server
                    .body
                    .config
                    .iter()
                    .find(|config| config.name == crate::config::CONFIG_FIELD_TRACING)
            });

            match trace_log {
                Some(tracing_config) => {
                    let trace_log = &tracing_config.value;

                    quote! {
                        if !(#trace_log).trace_log {
                            match (#logging_config).init(&Some(#trace_log)) {
                                Ok(()) => {},
                                Err(err) => {
                                    panic!(
                                        "g-server: failed initializing logging(outside tracing): {}",
                                        err
                                    );
                                }
                            }
                        }
                    }
                }
                None => {
                    quote! {
                        match (#logging_config).init(&None) {
                            Ok(()) => {},
                            Err(err) => {
                                panic!(
                                    "g-server: failed initializing logging: {}",
                                    err
                                );
                            }
                        }
                    }
                }
            }
        })
        .unwrap_or_else(|| quote! {});

    let tracing_init = if let Some(config) = servers.iter().find_map(|server| {
        server
            .body
            .config
            .iter()
            .find(|config| config.name == crate::config::CONFIG_FIELD_TRACING)
    }) {
        let tracing_config = &config.value;
        quote! {
            {
                match (#tracing_config).init() {
                    Ok(()) => {},
                    Err(err) => panic!("g-server: failed initializing tracing: {}", err),
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #[g_server::tokio::main(crate = "g_server::tokio")]
        async fn main() {
            #(#initializers)*

            #(#listeners)*

            #logging_init
            #tracing_init

            #grace_shutdown_canc_token

            #(#tls_config)*

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
        fn __register_global_middlewares<C, RLKey>(
            global_config: &::g_server::Config<RLKey>,
            mut router: g_server::axum::Router<C>,
        ) -> g_server::axum::Router<C>
        where
            C: Clone + Send + Sync + 'static,
            RLKey: std::fmt::Debug + Eq + PartialEq + Clone + std::hash::Hash + g_server::config::CustomKey + Send + Sync + 'static,
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

            if let Some(_) = global_config.tracing {
                if let Some(ref req_id) = global_config.request_id {
                    router = router.layer(
                        g_server::axum::middleware::from_fn_with_state(
                            Some(req_id.header.clone()),
                            tracing_middleware,
                        )
                    )
                } else {
                    router = router.layer(
                        g_server::axum::middleware::from_fn_with_state(
                            None,
                            tracing_middleware,
                        )
                    )
                }
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

            if let Some(ref rate_limit) = global_config.rate_limit {
                router = rate_limit.layer(router);
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
                router = router.layer(cors.clone().layer())
            }

            if let Some(ref req_id) = global_config.request_id {
                router = router.layer(
                    g_server::axum::middleware::from_fn_with_state(
                        req_id.clone(),
                        request_id_header_middleware,
                    )
                )
            }

            if let Some(fallback_err) = global_config.fallback_error {
                router = router.fallback(
                    move || async move {
                        fallback_err()
                    }
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
        fn __register_route_middlewares<C, RLKey>(
            config: &::g_server::Config<RLKey>,
            mut router: g_server::axum::routing::MethodRouter<C>,
        ) -> g_server::axum::routing::MethodRouter<C>
        where
            C: Clone + Send + Sync + 'static,
            RLKey: std::fmt::Debug + Eq + PartialEq + Clone + std::hash::Hash + g_server::config::CustomKey + Send + Sync + 'static,
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

            if let Some(_) = config.tracing {
                if let Some(ref req_id) = config.request_id {
                    router = router.route_layer(
                        g_server::axum::middleware::from_fn_with_state(
                            Some(req_id.header.clone()),
                            tracing_middleware,
                        )
                    )
                } else {
                    router = router.route_layer(
                        g_server::axum::middleware::from_fn_with_state(
                            None,
                            tracing_middleware,
                        )
                    )
                }
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

            if let Some(ref rate_limit) = config.rate_limit {
                router = rate_limit.route_layer(router);
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
                router = router.route_layer(cors.clone().layer())
            }

            if let Some(ref req_id) = config.request_id {
                router = router.route_layer(
                    g_server::axum::middleware::from_fn_with_state(
                        req_id.clone(),
                        request_id_header_middleware,
                    )
                )
            }

            if let Some(fallback_err) = config.fallback_error {
                router = router.fallback(
                    move || async move {
                        fallback_err()
                    }
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

fn request_id_header_middleware() -> TokenStream2 {
    quote! {
        pub(crate) async fn request_id_header_middleware(
            g_server::axum::extract::State(req_id): g_server::axum::extract::State<
                g_server::config::RequestId,
            >,
            mut request: g_server::axum::extract::Request,
            next: g_server::axum::middleware::Next,
        ) -> g_server::axum::response::Response {
            let request_id = match request.headers_mut().entry(&req_id.header) {
                g_server::http::header::Entry::Occupied(entry) => entry.get().clone(),
                g_server::http::header::Entry::Vacant(entry) => {
                    let value = match req_id.try_new_req_id() {
                        Ok(id) => id,
                        Err(err) => {
                            return (
                                g_server::StatusCode::INTERNAL_SERVER_ERROR,
                                format!("g-server: failed creating new request id: {}", err),
                            )
                                .into_response();
                        }
                    };
                    entry.insert(value.clone());
                    value
                }
            };

            let mut response = next.run(request).await;

            if let g_server::http::header::Entry::Vacant(entry) = response.headers_mut().entry(&req_id.header)
            {
                entry.insert(request_id);
            }

            response
        }
    }
}

fn tracing_middleware() -> TokenStream2 {
    quote! {
        pub(crate) async fn tracing_middleware(
            g_server::axum::extract::State(req_id_header): g_server::axum::extract::State<
                Option<g_server::http::HeaderName>,
            >,
            request: g_server::axum::extract::Request,
            next: g_server::axum::middleware::Next,
        ) -> g_server::axum::response::Response {
            use g_server::tracing::Instrument;

            let span = if let Some(req_id_header) = req_id_header {
                match request.headers().get(&req_id_header) {
                    Some(value) => g_server::tracing::info_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = %value.to_str().unwrap_or("<invalid>"),
                    ),
                    None => g_server::tracing::info_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                    ),
                }
            } else {
                g_server::tracing::info_span!(
                    "request",
                    method = %request.method(),
                    uri = %request.uri(),
                )
            };

            g_server::tracing::info!("::REQUEST_BEGIN::");

            let resp = next.run(request)
                .instrument(span)
                .await;

            g_server::tracing::info!("::REQUEST_END::");

            resp
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

// generate TLS server
#[cfg(feature = "tls")]
fn generate_tls_server(name: &Ident, is_graceful: bool) -> TokenStream2 {
    let tls_config_name = format_ident!("{}_tls_config", name);

    let tls_server = quote! {
        g_server::axum_server::bind_rustls(
            std::net::SocketAddr::new(
                #name.0.ip_address.parse().expect(format!("invalid ip address: {}", #name.0.ip_address).as_str()),
                #name.0.port,
            ),
            #tls_config_name.clone(),
        )
    };

    if is_graceful {
        return quote! {
            #tls_server.handle(handle.clone()).serve(#name.1.into_make_service_with_connect_info::<std::net::SocketAddr>())
        };
    }

    quote! {
        #tls_server.serve(#name.1.into_make_service_with_connect_info::<std::net::SocketAddr>())
    }
}
