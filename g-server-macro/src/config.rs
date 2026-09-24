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
pub(crate) const CONFIG_FIELD_CORS: &str = "cors";
pub(crate) const CONFIG_FIELD_FILE_DIR: &str = "dir";
pub(crate) const CONFIG_FIELD_FILE_FALLBACK_FILE: &str = "fallback_file";
pub(crate) const CONFIG_FIELD_FILE_EMBED: &str = "embed";

/// Configs that only allowed in server's root.
pub(crate) static GLOBAL_CONFIGS: &[&str] = &[CONFIG_FIELD_NORMALIZE_ENDPOINT];

pub(crate) static FILE_CONFIGS: &[&str] = &[
    CONFIG_FIELD_FILE_DIR,
    CONFIG_FIELD_FILE_FALLBACK_FILE,
    CONFIG_FIELD_FILE_EMBED,
];

pub(crate) static CUSTOM_ERRORS: &[&str] = &[
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

        let value: Expr = if name.to_string() == CONFIG_FIELD_CORS {
            parse_cors(input)?
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
            CONFIG_FIELD_CORS => Ok(()),

            // file configs validations
            CONFIG_FIELD_FILE_DIR | CONFIG_FIELD_FILE_FALLBACK_FILE => {
                Self::validate_string(&value)
            }
            CONFIG_FIELD_FILE_EMBED => Self::validate_bool(&value),

            // custom errors validations
            CONFIG_FIELD_TIMEOUT_ERROR
            | CONFIG_FIELD_CONCURRENCY_LIMIT_ERROR
            | CONFIG_FIELD_BAD_REQUEST_ERROR => Self::validate_custom_errors(&value),

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
