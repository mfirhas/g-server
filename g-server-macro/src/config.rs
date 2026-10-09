use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use std::{fmt::Display, str::FromStr};
use syn::{Expr, ExprArray, Result, Token, parse::ParseStream, spanned::Spanned};

pub(crate) const CONFIG_FIELD_TIMEOUT: &str = "timeout";
pub(crate) const CONFIG_FIELD_CONCURRENCY_LIMIT: &str = "concurrency_limit";
pub(crate) const CONFIG_FIELD_BODY_LIMIT: &str = "body_limit";
pub(crate) const CONFIG_FIELD_COMPRESSION: &str = "compression";
pub(crate) const CONFIG_FIELD_NORMALIZE_ENDPOINT: &str = "normalize_endpoint";
pub(crate) const CONFIG_FIELD_TIMEOUT_ERROR: &str = "timeout_error";
pub(crate) const CONFIG_FIELD_CONCURRENCY_LIMIT_ERROR: &str = "concurrency_limit_error";
pub(crate) const CONFIG_FIELD_BAD_REQUEST_ERROR: &str = "bad_request_error";
pub(crate) const CONFIG_FIELD_FALLBACK_ERROR: &str = "fallback_error";
pub(crate) const CONFIG_FIELD_REQUEST_ID: &str = "request_id";
pub(crate) const CONFIG_FIELD_CORS: &str = "cors";
pub(crate) const CONFIG_FIELD_RATE_LIMIT: &str = "rate_limit";
pub(crate) const CONFIG_FIELD_GRACEFUL_SHUTDOWN: &str = "graceful_shutdown";
pub(crate) const CONFIG_FIELD_FILE_DIR: &str = "dir";
pub(crate) const CONFIG_FIELD_FILE_FALLBACK_FILE: &str = "fallback_file";
pub(crate) const CONFIG_FIELD_FILE_EMBED: &str = "embed";

pub(crate) const CONFIG_FIELD_TLS: &str = "tls";

pub(crate) const CONFIG_FIELD_LOGGING: &str = "logging";
pub(crate) const CONFIG_FIELD_TRACING: &str = "tracing";

/// Configs that only allowed in server's root.
pub(crate) static GLOBAL_CONFIGS: &[&str] = &[
    CONFIG_FIELD_NORMALIZE_ENDPOINT,
    CONFIG_FIELD_GRACEFUL_SHUTDOWN,
    CONFIG_FIELD_TLS,
    CONFIG_FIELD_LOGGING,
    CONFIG_FIELD_TRACING,
];

pub(crate) static GLOBAL_GROUP_CONFIGS: &[&str] = &[CONFIG_FIELD_FALLBACK_ERROR];

pub(crate) static FILE_CONFIGS: &[&str] = &[
    CONFIG_FIELD_FILE_DIR,
    CONFIG_FIELD_FILE_FALLBACK_FILE,
    CONFIG_FIELD_FILE_EMBED,
];

pub(crate) static CUSTOM_ERRORS: &[&str] = &[
    CONFIG_FIELD_TIMEOUT_ERROR,
    CONFIG_FIELD_CONCURRENCY_LIMIT_ERROR,
    CONFIG_FIELD_BAD_REQUEST_ERROR,
    CONFIG_FIELD_FALLBACK_ERROR,
];

pub(crate) static ALL_INHERIT_CONFIGS: &[&str] = &[
    CONFIG_FIELD_TIMEOUT_ERROR,
    CONFIG_FIELD_CONCURRENCY_LIMIT_ERROR,
    CONFIG_FIELD_BAD_REQUEST_ERROR,
];

/// Parses config.
///
/// ```
/// gserver! {
///     ...
///     config: {
///         name: value, // parses this
///     }
///     ...
///     get: {
///         config: {
///             name: value, // and this
///         }
///     }
/// }
/// ```
///
/// Each config entry will be parsed into [`ConfigEntry`].
pub(crate) fn parse_config(input: ParseStream<'_>) -> Result<Vec<ConfigEntry>> {
    let mut entries = Vec::new();

    while !input.is_empty() {
        let name: Ident = input.parse()?;

        input.parse::<Token![:]>()?;

        let value: Expr = if name.to_string() == CONFIG_FIELD_REQUEST_ID {
            parse_request_id(&name, input)?
        } else if name.to_string() == CONFIG_FIELD_CORS {
            parse_cors(input)?
        } else if name.to_string() == CONFIG_FIELD_LOGGING {
            parse_logging(&name, input)?
        } else if name.to_string() == CONFIG_FIELD_TRACING {
            parse_tracing(&name, input)?
        } else if name.to_string() == CONFIG_FIELD_RATE_LIMIT {
            if !cfg!(feature = "ratelimit") {
                return Err(syn::Error::new(
                    name.span(),
                    "`ratelimit` requires feature `ratelimit`",
                ));
            }
            parse_rate_limit(&name, input)?
        } else if name.to_string() == CONFIG_FIELD_TLS {
            if !cfg!(feature = "tls") {
                return Err(syn::Error::new(name.span(), "`tls` requires feature `tls`"));
            }
            parse_tls(&name, input)?
        } else {
            input.parse()?
        };

        let config = ConfigEntry::try_new(name.clone(), value)
            .map_err(|err| syn::Error::new(name.span(), err.to_string()))?;

        entries.push(config);

        crate::consume_comma(input)?;
    }

    Ok(entries)
}

fn parse_request_id(ident: &Ident, input: ParseStream<'_>) -> Result<Expr> {
    let content;
    syn::braced!(content in input);

    let mut id: Option<Expr> = None;
    let mut header: Option<Expr> = None;

    while !content.is_empty() {
        let field: Ident = content.parse()?;

        content.parse::<Token![:]>()?;

        match field.to_string().as_str() {
            "id" => {
                let key_ident: Ident = content.parse()?;
                if key_ident.to_string() == "uuid_v4" {
                    id = Some(syn::parse_quote!(g_server::config::RequestIdType::UUIDv4));
                } else if key_ident.to_string() == "uuid_v7" {
                    id = Some(syn::parse_quote!(g_server::config::RequestIdType::UUIDv7));
                } else if key_ident.to_string() == "custom" {
                    let key_content;
                    syn::parenthesized!(key_content in content);
                    let val: syn::Path = key_content.parse()?;
                    let func = val
                        .segments
                        .last()
                        .expect("expect `fn() -> Result<crate::http::HeaderValue, String>`");
                    id = Some(syn::parse_quote!( g_server::config::RequestIdType::Custom(#func) ));
                } else {
                    return Err(syn::Error::new(
                        key_ident.span(),
                        "invalid `id` type: supported `uuid_v4`, `uuid_v7`, `fn() -> Result<crate::http::HeaderValue, String>`",
                    ));
                }
            }
            "header" => {
                let hdr = content.parse::<Expr>()?;
                match &hdr {
                    syn::Expr::Lit(expr_lit) => {
                        if let syn::Lit::Str(value) = &expr_lit.lit {
                            let value = value.value();
                            if !value
                                .chars()
                                .filter(|c| c.is_alphabetic())
                                .all(|c| c.is_lowercase())
                            {
                                return Err(syn::Error::new(
                                    hdr.span(),
                                    "`header` name must be declared as all lower-case",
                                ));
                            }
                            header = Some(
                                syn::parse_quote!(g_server::http::HeaderName::from_static(#value)),
                            );
                        }
                    }

                    syn::Expr::Path(expr_path) => {
                        let path = &expr_path
                            .path
                            .segments
                            .last()
                            .expect("expect valid const or static string");
                        header = Some(syn::parse_quote!(
                            {
                                if !#path.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_lowercase()) {
                                    panic!("g-server: `request_id` header name declaration must be all lower-case")
                                }
                                g_server::http::HeaderName::from_static(#path)
                            }
                        ))
                    }

                    _ => {
                        return Err(syn::Error::new(
                            ident.span(),
                            "`header` expects valid http header value from literal string, const, or static",
                        ));
                    }
                }
            }
            _ => {
                return Err(syn::Error::new(
                    field.span(),
                    format!("unknown `request_id` config `{field}`"),
                ));
            }
        }

        crate::consume_comma(&content)?;
    }

    match (id, header) {
        (Some(id), Some(header)) => {
            Ok(syn::parse_quote!(g_server::config::RequestId { id: #id, header: #header }))
        }
        (Some(id), None) => Ok(
            syn::parse_quote!(g_server::config::RequestId { id: #id, header: g_server::http::HeaderName::from_static("X-Request-Id") }),
        ),
        (None, Some(header)) => Ok(
            syn::parse_quote!(g_server::config::RequestId { id: g_server::config::RequestIdType::UUIDv4, header: #header }),
        ),
        _ => Ok(syn::parse_quote!(g_server::config::RequestId::default())),
    }
}

fn parse_cors(input: ParseStream<'_>) -> Result<Expr> {
    let content;
    syn::braced!(content in input);

    let mut origins = None;
    let mut methods = None;
    let mut headers = None;
    let mut exposed_headers = None;
    let mut credentials = None;
    let mut max_age = None;

    while !content.is_empty() {
        let field: Ident = content.parse()?;

        content.parse::<Token![:]>()?;

        match field.to_string().as_str() {
            "allowed_origins" => {
                origins = Some(content.parse::<ExprArray>()?);
            }
            "allowed_methods" => {
                methods = Some(content.parse::<ExprArray>()?);
            }
            "allowed_headers" => {
                headers = Some(content.parse::<ExprArray>()?);
            }
            "exposed_headers" => {
                exposed_headers = Some(content.parse::<ExprArray>()?);
            }
            "allow_credentials" => {
                credentials = Some(content.parse::<Expr>()?);
            }
            "max_age" => {
                max_age = Some(content.parse::<Expr>()?);
            }
            _ => {
                return Err(syn::Error::new(
                    field.span(),
                    format!("unknown CORS option `{field}`"),
                ));
            }
        }

        crate::consume_comma(&content)?;
    }

    let allowed_origins = if let Some(origins) = origins {
        expr_array_to_header_values(&origins)?.to_token_stream()
    } else {
        quote! { None }
    };

    let allowed_methods = if let Some(methods_expr_arr) = methods {
        parse_cors_methods(methods_expr_arr)?
    } else {
        quote! { None }
    };

    let allowed_headers = if let Some(headers) = headers {
        expr_array_to_header_names(&headers)?.to_token_stream()
    } else {
        quote! { None }
    };

    let exposed_headers = if let Some(exposed_headers) = exposed_headers {
        expr_array_to_header_names(&exposed_headers)?.to_token_stream()
    } else {
        quote! { None }
    };

    let allow_credentials = credentials
        .map(|value| quote! { Some(#value) })
        .unwrap_or_else(|| quote! { None });

    let max_age = max_age
        .map(|value| quote! { Some(#value) })
        .unwrap_or_else(|| quote! { None });

    Ok(syn::parse_quote! {
        ::g_server::config::Cors {
            allowed_origins: #allowed_origins,

            allowed_methods: #allowed_methods,

            allowed_headers: #allowed_headers,

            exposed_headers: #exposed_headers,

            allow_credentials: #allow_credentials,
            max_age: #max_age,
        }
    })
}

fn parse_cors_methods(array: ExprArray) -> syn::Result<TokenStream2> {
    let methods = array
        .elems
        .into_iter()
        .map(|expr| {
            let Expr::Path(path) = expr else {
                return Err(syn::Error::new_spanned(
                    expr,
                    "CORS method must be an HTTP method identifier",
                ));
            };

            let Some(segment) = path.path.segments.last() else {
                return Err(syn::Error::new_spanned(
                    path,
                    "CORS method must be an HTTP method identifier",
                ));
            };

            let method = match segment.ident.to_string().as_str() {
                "GET" | "Get" | "get" => quote! { ::g_server::http::Method::GET },
                "POST" | "Post" | "post" => quote! { ::g_server::http::Method::POST },
                "PUT" | "Put" | "put" => quote! { ::g_server::http::Method::PUT },
                "DELETE" | "Delete" | "delete" => quote! { ::g_server::http::Method::DELETE },
                "PATCH" | "Patch" | "patch" => quote! { ::g_server::http::Method::PATCH },
                "HEAD" | "Head" | "head" => quote! { ::g_server::http::Method::HEAD },
                "OPTIONS" | "Options" | "options" => quote! { ::g_server::http::Method::OPTIONS },
                "CONNECT" | "Connect" | "connect" => quote! { ::g_server::http::Method::CONNECT },
                "TRACE" | "Trace" | "trace" => quote! { ::g_server::http::Method::TRACE },
                "QUERY" | "Query" | "query" => quote! { ::g_server::http::Method::QUERY },
                "ANY" | "Any" | "any" => quote! { ::g_server::http::Method::ANY },
                _ => {
                    return Err(syn::Error::new_spanned(segment, "invalid CORS HTTP method"));
                }
            };

            Ok(method)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(if methods.is_empty() {
        quote! {None}
    } else {
        quote! {
            Some(vec![
                #(#methods),*
            ])
        }
    })
}

fn expr_array_to_header_values(arr: &ExprArray) -> syn::Result<Expr> {
    let strings: Vec<String> = arr
        .elems
        .iter()
        .map(|expr| match expr {
            Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Str(s) => Ok(s.value()),
                other => Err(syn::Error::new_spanned(other, "expected a string literal")),
            },
            other => Err(syn::Error::new_spanned(other, "expected a string literal")),
        })
        .collect::<syn::Result<_>>()?;

    let expr = if !strings.is_empty() {
        syn::parse_quote! { Some(vec![#(#strings.to_string()),*].iter()
        .map(|origin| {
            origin
                .parse::<g_server::http::HeaderValue>()
                .expect("invalid CORS allowed origin")
        })
        .collect::<Vec<_>>()) }
    } else {
        syn::parse_quote! { None }
    };

    Ok(expr)
}

fn expr_array_to_header_names(arr: &ExprArray) -> syn::Result<Expr> {
    let strings: Vec<String> = arr
        .elems
        .iter()
        .map(|expr| match expr {
            Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Str(s) => Ok(s.value()),
                other => Err(syn::Error::new_spanned(other, "expected a string literal")),
            },
            other => Err(syn::Error::new_spanned(other, "expected a string literal")),
        })
        .collect::<syn::Result<_>>()?;

    let expr = if !strings.is_empty() {
        syn::parse_quote! { Some(vec![#(#strings.to_string()),*].iter()
        .map(|origin| {
            origin
                .parse::<g_server::http::HeaderName>()
                .expect("invalid CORS allowed origin")
        })
        .collect::<Vec<_>>()) }
    } else {
        syn::parse_quote! { None }
    };

    Ok(expr)
}

const RATE_LIMIT_KEY_GLOBAL: &str = "global";
const RATE_LIMIT_KEY_IP: &str = "ip";
const RATE_LIMIT_KEY_CUSTOM: &str = "custom";

fn parse_rate_limit(ident: &Ident, input: ParseStream<'_>) -> Result<Expr> {
    let content;
    syn::braced!(content in input);

    let mut burst_size = None;
    let mut interval = None;
    let mut with_headers = None;
    let mut key: Option<(Ident, Expr)> = None;

    while !content.is_empty() {
        let field: Ident = content.parse()?;

        content.parse::<Token![:]>()?;

        match field.to_string().as_str() {
            "burst_size" => burst_size = Some(content.parse::<Expr>()?),
            "interval" => interval = Some(content.parse::<Expr>()?),
            "with_headers" => with_headers = Some(content.parse::<Expr>()?),
            "key" => {
                let key_ident: Ident = content.parse()?;
                if key_ident.to_string() == RATE_LIMIT_KEY_GLOBAL
                    || key_ident.to_string() == RATE_LIMIT_KEY_IP
                {
                    key = Some((key_ident, syn::parse_quote!(())));
                } else if key_ident.to_string() == RATE_LIMIT_KEY_CUSTOM {
                    let key_content;
                    syn::parenthesized!(key_content in content);
                    let val: Expr = key_content.parse()?;
                    key = Some((key_ident, val));
                } else {
                    return Err(syn::Error::new(
                        key_ident.span(),
                        "invalid `key` type: supported `global`, `ip`, `custom(Type: CustomKey)`",
                    ));
                }
            }
            _ => {
                return Err(syn::Error::new(
                    field.span(),
                    format!("unknown rate limit option `{field}`"),
                ));
            }
        }

        crate::consume_comma(&content)?;
    }

    let burst_size = if let Some(burst) = burst_size {
        match &burst {
            Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(lit_int),
                ..
            }) => {
                let b: u32 = lit_int.base10_parse()?;
                if b == 0 {
                    return Err(syn::Error::new(
                        burst.span(),
                        "`burst_size` must not be zero",
                    ));
                }
            }
            _ => return Err(syn::Error::new(burst.span(), "expects an integer")),
        };
        burst
    } else {
        return Err(syn::Error::new(
            ident.span(),
            "rate limit burst_size must not be empty",
        ));
    };

    let interval = if let Some(interval) = interval {
        match &interval {
            Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(lit_int),
                ..
            }) => {
                let p: u64 = lit_int.base10_parse()?;
                if p == 0 {
                    return Err(syn::Error::new(
                        interval.span(),
                        "`interval` must not be zero",
                    ));
                }
            }
            _ => return Err(syn::Error::new(interval.span(), "expects an integer")),
        };
        interval
    } else {
        // default to refilling token at every 100 milliseconds.
        syn::parse_quote!(100)
    };

    let with_headers = if let Some(use_headers) = with_headers {
        ConfigEntry::validate_bool(&use_headers)?;
        use_headers
    } else {
        syn::parse_quote!(false)
    };

    let mut rate_limit_key: Expr = syn::parse_quote! { g_server::config::RateLimitKey::default() };
    if let Some((ident, custom_key)) = key {
        if ident.to_string() == RATE_LIMIT_KEY_GLOBAL {
            rate_limit_key = syn::parse_quote! { g_server::config::RateLimitKey::Global };
        } else if ident.to_string() == RATE_LIMIT_KEY_CUSTOM {
            rate_limit_key =
                syn::parse_quote! { g_server::config::RateLimitKey::Custom(#custom_key) }
        }
    }

    Ok(syn::parse_quote! {
        g_server::config::RateLimit {
            burst_size: #burst_size,
            interval: #interval,
            with_headers: #with_headers,
            key: #rate_limit_key,
        }
    })
}

fn parse_tls(ident: &Ident, input: ParseStream<'_>) -> Result<Expr> {
    let content;
    syn::braced!(content in input);

    let mut cert = None;
    let mut key = None;
    let mut redirect_from_port = None;
    let mut client_cas = None;

    while !content.is_empty() {
        let field: Ident = content.parse()?;

        content.parse::<Token![:]>()?;

        match field.to_string().as_str() {
            "cert" => {
                let cert_expr = content.parse::<Expr>()?;
                match &cert_expr {
                    Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) => {
                        let path_str = lit_str.value();
                        let path = std::path::Path::new(&path_str);
                        if !path.is_file() || !path.exists() {
                            return Err(syn::Error::new(
                                cert_expr.span(),
                                "`cert` file doesn't exist",
                            ));
                        }
                    }
                    Expr::Path(syn::ExprPath { .. }) => {}
                    _ => {
                        return Err(syn::Error::new(
                            cert_expr.span(),
                            "`cert` accepts literal string, static or const",
                        ));
                    }
                }
                cert = Some(cert_expr)
            }
            "key" => {
                let key_expr = content.parse::<Expr>()?;
                match &key_expr {
                    Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) => {
                        let path_str = lit_str.value();
                        let path = std::path::Path::new(&path_str);
                        if !path.is_file() || !path.exists() {
                            return Err(syn::Error::new(
                                key_expr.span(),
                                "`key` file doesn't exist",
                            ));
                        }
                    }
                    Expr::Path(syn::ExprPath { .. }) => {}
                    _ => {
                        return Err(syn::Error::new(
                            key_expr.span(),
                            "`cert` accepts literal string, static or const",
                        ));
                    }
                }
                key = Some(key_expr)
            }
            "redirect_from" => {
                redirect_from_port = Some(content.parse::<Expr>()?);
            }
            "client_cas" => {
                let cas = content.parse::<ExprArray>()?;
                for ca in cas.elems.iter() {
                    match ca {
                        Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit_str),
                            ..
                        }) => {
                            let path_str = lit_str.value();
                            let path = std::path::Path::new(&path_str);
                            if !path.is_file() || !path.exists() {
                                return Err(syn::Error::new(
                                    ca.span(),
                                    format!("\"{}\" file doesn't exist", lit_str.value()),
                                ));
                            }
                        }
                        Expr::Path(syn::ExprPath { .. }) => {}
                        _ => {
                            return Err(syn::Error::new(
                                ca.span(),
                                "client ca certs accept literal string, static or const",
                            ));
                        }
                    }
                }
                client_cas = Some(cas);
            }
            _ => {
                return Err(syn::Error::new(
                    field.span(),
                    format!("unknown tls config `{field}`"),
                ));
            }
        }

        crate::consume_comma(&content)?;
    }

    let (cert, key) = match (cert, key) {
        (Some(cert), Some(key)) => (cert, key),
        _ => {
            return Err(syn::Error::new(
                ident.span(),
                "`cert` and `key` must be present",
            ));
        }
    };

    // 1 server cannot serve public TLS and mTLS at the same time
    if redirect_from_port.is_some() && client_cas.is_some() {
        return Err(syn::Error::new(
            ident.span(),
            "`redirect_from` is only for public TLS and `client_cas` is only for mTLS, both cannot co-exist together.",
        ));
    }

    let redirect_port = if let Some(port) = redirect_from_port {
        quote! { redirect_from_port: Some(#port), }
    } else {
        quote! { redirect_from_port: None, }
    };

    let client_cas = if let Some(client_cas) = client_cas {
        if client_cas.elems.is_empty() {
            return Err(syn::Error::new(
                ident.span(),
                "`client_cas` may not be empty for mTLS configuration",
            ));
        }
        let elements = client_cas.elems.iter().map(|expr| {
            quote! {
                #expr.to_string()
            }
        });

        quote! {
            client_cas: Some(vec![
                #(#elements),*
            ])
        }
    } else {
        quote! {
            client_cas: None,
        }
    };

    Ok(syn::parse_quote! {
        g_server::config::Tls {
            cert: #cert.to_string(),
            key: #key.to_string(),
            #redirect_port
            #client_cas
        }
    })
}

fn parse_logging(ident: &Ident, input: ParseStream<'_>) -> Result<Expr> {
    let content;
    syn::braced!(content in input);

    let mut level: Option<Expr> = None;
    let mut format: Option<Expr> = None;
    let mut time_offset: Option<Expr> = None;
    let mut init: Option<syn::Path> = None;
    let mut target: Option<syn::Expr> = None;

    while !content.is_empty() {
        let field: Ident = content.parse()?;

        content.parse::<Token![:]>()?;

        match field.to_string().as_str() {
            "level" => {
                let level_ident: Ident = content.parse()?;
                match level_ident.to_string().as_str() {
                    "error" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Error)),
                    "warn" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Warn)),
                    "info" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Info)),
                    "debug" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Debug)),
                    "trace" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Trace)),
                    _ => {
                        return Err(syn::Error::new(
                            ident.span(),
                            "invalid tracing max level, expected: error, warn, info, debug, or trace (in order from left to right)",
                        ));
                    }
                }
            }
            "format" => {
                let format_ident: Ident = content.parse()?;
                match format_ident.to_string().as_str() {
                    "default" => {
                        format = Some(syn::parse_quote!(g_server::config::LogFormat::Default))
                    }
                    "pretty" => {
                        format = Some(syn::parse_quote!(g_server::config::LogFormat::Pretty))
                    }
                    "json" => format = Some(syn::parse_quote!(g_server::config::LogFormat::Json)),
                    _ => {
                        return Err(syn::Error::new(
                            ident.span(),
                            "invalid tracing format, expected: default, pretty, or json",
                        ));
                    }
                }
            }
            "time_offset" => {
                let time_offset_ident: Ident = content.parse()?;
                match time_offset_ident.to_string().as_str() {
                    "utc" => {
                        time_offset = Some(syn::parse_quote!(g_server::config::LogTimeOffset::UTC))
                    }
                    "local" => {
                        time_offset =
                            Some(syn::parse_quote!(g_server::config::LogTimeOffset::Local))
                    }
                    _ => {
                        return Err(syn::Error::new(
                            ident.span(),
                            "invalid tracing format, expected: default, pretty, or json",
                        ));
                    }
                }
            }

            "target" => {
                let target_ident: Ident = content.parse()?;
                match target_ident.to_string().as_str() {
                    "stdout" => {
                        target = Some(syn::parse_quote!(g_server::config::LogOutput::StdOut))
                    }
                    "stderr" => {
                        target = Some(syn::parse_quote!(g_server::config::LogOutput::StdErr))
                    }
                    "file" => {
                        let path_content;
                        syn::parenthesized!(path_content in content);

                        // Any expression evaluating to a string: literal, const, concat!(..), env!(..)
                        let path: syn::Expr = path_content.parse()?;

                        if !path_content.is_empty() {
                            return Err(
                                path_content.error("`file(..)` takes exactly one path expression")
                            );
                        }

                        target = Some(syn::parse_quote!(
                            g_server::config::LogOutput::File(std::path::PathBuf::from(#path))
                        ))
                    }
                    _ => {
                        return Err(syn::Error::new(
                            target_ident.span(),
                            "invalid target, expected: stdout, stderr, or file(..)",
                        ));
                    }
                }
            }

            "init" => {
                let init_path: syn::Path = content.parse()?;
                init = Some(init_path)
            }

            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "invalid tracing config, expected: level, format or time_offset",
                ));
            }
        }
        crate::consume_comma(&content)?;
    }

    let level = level.unwrap_or(syn::parse_quote!(g_server::config::LogLevel::default()));
    let format = format.unwrap_or(syn::parse_quote!(g_server::config::LogFormat::default()));
    let time_offset =
        time_offset.unwrap_or(syn::parse_quote!(g_server::config::LogTimeOffset::default()));
    let target = target.unwrap_or(syn::parse_quote!(g_server::config::LogOutput::default()));
    let init_fn = init.unwrap_or(syn::parse_quote!(g_server::config::default_logger_init));

    Ok(syn::parse_quote! {
        g_server::config::Logging {
            level: #level,
            format: #format,
            time_offset: #time_offset,
            target: #target,
            init_fn: #init_fn,
        }
    })
}

fn parse_tracing(ident: &Ident, input: ParseStream<'_>) -> Result<Expr> {
    let content;
    syn::braced!(content in input);

    let mut level: Option<Expr> = None;
    let mut format: Option<Expr> = None;
    let mut time_offset: Option<Expr> = None;
    let mut target: Option<syn::Expr> = None;
    let mut trace_log: Option<Expr> = None;

    while !content.is_empty() {
        let field: Ident = content.parse()?;

        content.parse::<Token![:]>()?;

        match field.to_string().as_str() {
            "level" => {
                let level_ident: Ident = content.parse()?;
                match level_ident.to_string().as_str() {
                    "error" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Error)),
                    "warn" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Warn)),
                    "info" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Info)),
                    "debug" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Debug)),
                    "trace" => level = Some(syn::parse_quote!(g_server::config::LogLevel::Trace)),
                    _ => {
                        return Err(syn::Error::new(
                            ident.span(),
                            "invalid tracing max level, expected: error, warn, info, debug, or trace (in order from left to right)",
                        ));
                    }
                }
            }
            "format" => {
                let format_ident: Ident = content.parse()?;
                match format_ident.to_string().as_str() {
                    "default" => {
                        format = Some(syn::parse_quote!(g_server::config::LogFormat::Default))
                    }
                    "pretty" => {
                        format = Some(syn::parse_quote!(g_server::config::LogFormat::Pretty))
                    }
                    "json" => format = Some(syn::parse_quote!(g_server::config::LogFormat::Json)),
                    _ => {
                        return Err(syn::Error::new(
                            ident.span(),
                            "invalid tracing format, expected: default, pretty, or json",
                        ));
                    }
                }
            }
            "time_offset" => {
                let time_offset_ident: Ident = content.parse()?;
                match time_offset_ident.to_string().as_str() {
                    "utc" => {
                        time_offset = Some(syn::parse_quote!(g_server::config::LogTimeOffset::UTC))
                    }
                    "local" => {
                        time_offset =
                            Some(syn::parse_quote!(g_server::config::LogTimeOffset::Local))
                    }
                    _ => {
                        return Err(syn::Error::new(
                            ident.span(),
                            "invalid tracing format, expected: default, pretty, or json",
                        ));
                    }
                }
            }

            "target" => {
                let target_ident: Ident = content.parse()?;
                match target_ident.to_string().as_str() {
                    "stdout" => {
                        target = Some(syn::parse_quote!(g_server::config::LogOutput::StdOut))
                    }
                    "stderr" => {
                        target = Some(syn::parse_quote!(g_server::config::LogOutput::StdErr))
                    }
                    "file" => {
                        let path_content;
                        syn::parenthesized!(path_content in content);

                        // Any expression evaluating to a string: literal, const, concat!(..), env!(..)
                        let path: syn::Expr = path_content.parse()?;

                        if !path_content.is_empty() {
                            return Err(
                                path_content.error("`file(..)` takes exactly one path expression")
                            );
                        }

                        target = Some(syn::parse_quote!(
                            g_server::config::LogOutput::File(std::path::PathBuf::from(#path))
                        ))
                    }
                    _ => {
                        return Err(syn::Error::new(
                            target_ident.span(),
                            "invalid target, expected: stdout, stderr, or file(..)",
                        ));
                    }
                }
            }

            "trace_log" => {
                let trace_log_expr: Expr = content.parse()?;
                trace_log = Some(trace_log_expr)
            }

            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    "invalid tracing config, expected: level, format or time_offset",
                ));
            }
        }
        crate::consume_comma(&content)?;
    }

    let level = level.unwrap_or(syn::parse_quote!(g_server::config::LogLevel::default()));
    let format = format.unwrap_or(syn::parse_quote!(g_server::config::LogFormat::default()));
    let time_offset =
        time_offset.unwrap_or(syn::parse_quote!(g_server::config::LogTimeOffset::default()));
    let target = target.unwrap_or(syn::parse_quote!(g_server::config::LogOutput::default()));
    let trace_log = trace_log.unwrap_or(syn::parse_quote!(false));

    Ok(syn::parse_quote! {
        g_server::config::Tracing {
            level: #level,
            format: #format,
            time_offset: #time_offset,
            target: #target,
            trace_log: #trace_log,
        }
    })
}

/// validate config entries that depend on other config entries.
// fn validate_dependent_fields(entries: &[ConfigEntry]) -> Result<()> {
//     let pairs = [
//         (CONFIG_FIELD_TIMEOUT_ERROR, CONFIG_FIELD_TIMEOUT),
//         (CONFIG_FIELD_CONCURRENCY_LIMIT_ERROR, CONFIG_FIELD_CONCURRENCY_LIMIT),
//     ];

//     for (dependent, required) in pairs {
//         if let Some(entry) = entries.iter().find(|e| e.name == dependent) {
//             if !entries.iter().any(|e| e.name == required) {
//                 return Err(syn::Error::new(
//                     entry.name.span(),
//                     format!("`{dependent}` requires `{required}` to also be set"),
//                 ));
//             }
//         }
//     }

//     Ok(())
// }

/// Generation function for global config.
pub(crate) fn generate_global_config(entries: &[ConfigEntry]) -> TokenStream2 {
    let assignments = entries.iter().map(|entry| {
        let field = &entry.name;

        let value = &entry.value;

        if CUSTOM_ERRORS.contains(&field.to_string().as_str()) {
            if let Expr::Call(call) = value
                && let Expr::Path(p) = &*call.func
            {
                match p.path.get_ident().map(|i| i.to_string()).as_deref() {
                    Some("Json") | Some("json") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `json(g_server::Response<T: ::serde::Serialize>)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                            return quote! {
                                global_config.#field = Some(|err: &str| Into::<Response<_>>::into(#err_resp).bad_request_err_msg(err).into_axum_json());
                            }
                        } else {
                            return quote! {
                                global_config.#field = Some(|| Into::<Response<_>>::into(#err_resp).into_axum_json());
                            }
                        }
                    }
                    Some("Text") | Some("String") | Some("text") | Some("string") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `text(g_server::Response<T: Display>)`, or `string(...)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                            return quote! {
                                global_config.#field = Some(|err: &str| Into::<Response<_>>::into(#err_resp).bad_request_err_msg(err).into_axum_string());
                            }
                        } else {
                            return quote! {
                                global_config.#field = Some(|| Into::<Response<_>>::into(#err_resp).into_axum_string());
                            }
                        }
                    },
                    Some("Html") | Some("html") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `html(g_server::Response<T: Display>)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                            return quote! {
                                global_config.#field = Some(|err: &str| Into::<Response<_>>::into(#err_resp).bad_request_err_msg(err).into_axum_html());
                            }
                        } else {
                            return quote! {
                                global_config.#field = Some(|| Into::<Response<_>>::into(#err_resp).into_axum_html());
                            }
                        }
                    },
                    _ => {
                        if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                            return quote! {
                                global_config.#field = Some(|_: &str| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                            }
                        } else {
                            return quote! {
                                global_config.#field = Some(|| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                            }
                        }
                    }
                }
            }

            if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                return quote! {
                    global_config.#field = Some(|_: &str| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                }
            } else {
                return quote! {
                    global_config.#field = Some(|| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                }
            }
        }

        if field.to_string() == CONFIG_FIELD_RATE_LIMIT {
            return quote! {
                let mut global_config = global_config.with_rate_limit(#value);
            }
        }

        quote! {
            global_config.#field = (#value).into();
        }
    });

    quote! {
        let mut global_config = g_server::Config::empty();
        #(#assignments)*
    }
}

/// Generation function for route config.
pub(crate) fn generate_route_config(entries: &[ConfigEntry]) -> TokenStream2 {
    let assignments = entries.iter().map(|entry| {
        let field = &entry.name;

        let value = &entry.value;

        if CUSTOM_ERRORS.contains(&field.to_string().as_str()) {
            if let Expr::Call(call) = value
                && let Expr::Path(p) = &*call.func
            {
                match p.path.get_ident().map(|i| i.to_string()).as_deref() {
                    Some("Json") | Some("json") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `json(g_server::Response<T: ::serde::Serialize>)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                            return quote! {
                                config.#field = Some(|err: &str| Into::<Response<_>>::into(#err_resp).bad_request_err_msg(err).into_axum_json());
                            }
                        } else {
                            return quote! {
                                config.#field = Some(|| Into::<Response<_>>::into(#err_resp).into_axum_json());
                            }
                        }
                    }
                    Some("Text") | Some("String") | Some("text") | Some("string") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `text(g_server::Response<T: Display>)`, or `string(...)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                            return quote! {
                                config.#field = Some(|err: &str| Into::<Response<_>>::into(#err_resp).bad_request_err_msg(err).into_axum_string());
                            }
                        } else {
                            return quote! {
                                config.#field = Some(|| Into::<Response<_>>::into(#err_resp).into_axum_string());
                            }
                        }
                    },
                    Some("Html") | Some("html") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `html(g_server::Response<T: Display>)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                            return quote! {
                                config.#field = Some(|err: &str| Into::<Response<_>>::into(#err_resp).bad_request_err_msg(err).into_axum_html());
                            }
                        } else {
                            return quote! {
                                config.#field = Some(|| Into::<Response<_>>::into(#err_resp).into_axum_html());
                            }
                        }
                    },
                    _ => {
                        if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                            return quote! {
                                config.#field = Some(|_: &str| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                            }
                        } else {
                            return quote! {
                                config.#field = Some(|| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                            }
                        }
                    }
                }
            }

            if field.to_string().as_str() == CONFIG_FIELD_BAD_REQUEST_ERROR {
                return quote! {
                    config.#field = Some(|_: &str| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                }
            } else {
                return quote! {
                    config.#field = Some(|| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                }
            }
        }

        if field.to_string() == CONFIG_FIELD_RATE_LIMIT {
            return quote! {
                let mut config = config.with_rate_limit(#value);
            }
        }

        quote! {
            config.#field = (#value).into();
        }
    });

    quote! {
        let mut config = g_server::Config::empty();
        #(#assignments)*
    }
}

/// Represents config entry.
///
/// ```
/// gserver! {
///     ...
///     config: {
///         name: value,
///         ...
///     }
///     ...
///
///     get: {
///         config: {
///             name: value,
///             ...
///         },
///         ...
///     }
/// }
/// ```
///
/// `name` is from supported configs, from [`Compression`].
///
/// `value` is from supported value for each config entry.
///
/// This config entry will be mapped into `g_server::Config`.
#[derive(Clone)]
pub(crate) struct ConfigEntry {
    pub(crate) name: Ident,
    pub(crate) value: Expr,
}

impl ConfigEntry {
    pub(crate) fn try_new(name: Ident, mut value: Expr) -> Result<Self> {
        match name.to_string().as_str() {
            CONFIG_FIELD_TIMEOUT => Self::validate_integer(&value),
            CONFIG_FIELD_CONCURRENCY_LIMIT => Self::validate_integer(&value),
            CONFIG_FIELD_BODY_LIMIT => Self::validate_integer(&value),
            CONFIG_FIELD_COMPRESSION => Self::validate_compression(&mut value),
            CONFIG_FIELD_NORMALIZE_ENDPOINT => Self::validate_bool(&value),
            CONFIG_FIELD_GRACEFUL_SHUTDOWN => Self::validate_bool(&value),
            CONFIG_FIELD_REQUEST_ID => Ok(()),
            CONFIG_FIELD_CORS => Ok(()),
            CONFIG_FIELD_RATE_LIMIT => Ok(()),

            CONFIG_FIELD_TLS => Ok(()),

            CONFIG_FIELD_LOGGING => Ok(()),
            CONFIG_FIELD_TRACING => Ok(()),

            // file configs validations
            CONFIG_FIELD_FILE_DIR | CONFIG_FIELD_FILE_FALLBACK_FILE => {
                Self::validate_string(&value)
            }
            CONFIG_FIELD_FILE_EMBED => Self::validate_bool(&value),

            // custom errors validations
            CONFIG_FIELD_TIMEOUT_ERROR
            | CONFIG_FIELD_CONCURRENCY_LIMIT_ERROR
            | CONFIG_FIELD_BAD_REQUEST_ERROR
            | CONFIG_FIELD_FALLBACK_ERROR => Self::validate_custom_errors(&value),

            _ => Err(syn::Error::new(
                name.span(),
                format!("unknown config `{}`", name),
            )),
        }?;

        Ok(Self { name, value })
    }

    fn validate_integer(value: &Expr) -> Result<()> {
        match value {
            Expr::Lit(expr) if matches!(&expr.lit, syn::Lit::Int(_)) => Ok(()),

            _ => Err(syn::Error::new(value.span(), "expects an integer")),
        }
    }

    fn validate_string(value: &Expr) -> Result<()> {
        match value {
            Expr::Lit(expr) if matches!(&expr.lit, syn::Lit::Str(_)) => Ok(()),

            _ => Err(syn::Error::new(value.span(), "expects a string")),
        }
    }

    fn validate_compression(value: &mut Expr) -> Result<()> {
        let value_str = value.to_token_stream().to_string();
        let c = Compression::from_str(value_str.as_str())
            .map_err(|err| syn::Error::new(Span::call_site(), err))?;
        let c_ident = syn::Ident::new(c.to_string().as_str(), Span::call_site());
        *value = syn::parse2(quote! { g_server::Compression::#c_ident })?;

        Ok(())
    }

    fn validate_bool(value: &Expr) -> Result<()> {
        match value {
            Expr::Lit(expr) if matches!(&expr.lit, syn::Lit::Bool(_)) => Ok(()),

            _ => Err(syn::Error::new(value.span(), "expects a boolean")),
        }
    }

    fn validate_custom_errors(value: &Expr) -> Result<()> {
        if let Expr::Call(call) = value
            && let Expr::Path(p) = &*call.func
        {
            match p.path.get_ident().map(|i| i.to_string()).as_deref() {
                Some("Json") | Some("json") => {}
                Some("Text") | Some("String") | Some("text") | Some("string") => {}
                Some("Html") | Some("html") => {}
                _ => {
                    return Err(syn::Error::new(
                        value.span(),
                        "expected values: json(T), text(T), html(T), or ()",
                    ));
                }
            }
        } else {
            return Err(syn::Error::new(
                value.span(),
                "invalid custom errors values, expected values: json(T), text(T), html(T)",
            ));
        }

        Ok(())
    }
}

// COMPRESSION config
// --------------------------------------------------------------

#[derive(Clone, Copy)]
pub(crate) enum Compression {
    Deflate,
    Gzip,
    Brotli,
    Zstd,
    All,
}

impl Compression {
    pub(crate) fn display_list() -> String {
        String::from("deflate, gzip, brotli, zstd, all")
    }
}

impl Display for Compression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deflate => write!(f, "Deflate"),
            Self::Gzip => write!(f, "Gzip"),
            Self::Brotli => write!(f, "Brotli"),
            Self::Zstd => write!(f, "Zstd"),
            Self::All => write!(f, "All"),
        }
    }
}

impl FromStr for Compression {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Deflate" | "deflate" => Ok(Self::Deflate),
            "Gzip" | "gzip" => Ok(Self::Gzip),
            "Brotli" | "brotli" => Ok(Self::Brotli),
            "Zstd" | "zstd" => Ok(Self::Zstd),
            "All" | "all" => Ok(Self::All),
            other => Err(format!(
                "`{}` is invalid/unsupported compression method, supported: {}",
                other,
                Self::display_list()
            )),
        }
    }
}
