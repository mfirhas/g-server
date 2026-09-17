use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use std::{fmt::Display, str::FromStr};
use syn::{Expr, Result, Token, parse::ParseStream, spanned::Spanned};

pub(crate) const CONFIG_FIELD_TIMEOUT: &str = "timeout";
pub(crate) const CONFIG_FIELD_CONCURRENCY_LIMIT: &str = "concurrency_limit";
pub(crate) const CONFIG_FIELD_BODY_LIMIT: &str = "body_limit";
pub(crate) const CONFIG_FIELD_COMPRESSION: &str = "compression";
pub(crate) const CONFIG_FIELD_NORMALIZE_ENDPOINT: &str = "normalize_endpoint";
pub(crate) const CONFIG_FIELD_TIMEOUT_ERROR: &str = "timeout_error";
pub(crate) const CONFIG_FIELD_CONCURRENCY_LIMIT_ERROR: &str = "concurrency_limit_error";

/// Configs that only allowed in server's root.
pub(crate) static GLOBAL_CONFIGS: &[&str] = &[
    CONFIG_FIELD_NORMALIZE_ENDPOINT,
];

pub(crate) static CUSTOM_ERRORS: &[&str] = &[
    CONFIG_FIELD_TIMEOUT_ERROR,
    CONFIG_FIELD_CONCURRENCY_LIMIT_ERROR,
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

        let value: Expr = input.parse()?;

        let config = ConfigEntry::try_new(name.clone(), value)
            .map_err(|err| syn::Error::new(name.span(), err.to_string()))?;

        entries.push(config);

        crate::consume_comma(input)?;
    }

    Ok(entries)
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
                        return quote! {
                            global_config.#field = Some(|| #err_resp.into_axum_json());
                        };
                    }
                    Some("Text") | Some("String") | Some("text") | Some("string") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `text(g_server::Response<T: ::serde::Serialize>)`, or `string(...)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        return quote! {
                            global_config.#field = Some(|| #err_resp.into_axum_string());                  
                        };
                    },
                    Some("Html") | Some("html") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `html(g_server::Response<T: ::serde::Serialize>)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        return quote! {
                            global_config.#field = Some(|| #err_resp.into_axum_html()); 
                        };
                    },
                    _ => {
                        // should be unreachable if validated properly.
                        return quote! {
                            global_config.#field = Some(|| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                        }
                    }
                }
            }

            return quote! {
                config.#field = Some(|| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
            };
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
                        return quote! {
                            config.#field = Some(|| #err_resp.into_axum_json());
                        };
                    }
                    Some("Text") | Some("String") | Some("text") | Some("string") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `text(g_server::Response<T: Display>)`, or `string(...)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        return quote! {
                            config.#field = Some(|| #err_resp.into_axum_string()); 
                        };
                    },
                    Some("Html") | Some("html") => {
                        let err_resp: &Expr = &call.args.get(0).expect(
                            format!(
                                "root config field `{}` requires value of `html(g_server::Response<T: Display>)`", &field.to_string().as_str()
                            )
                            .as_str(),
                        );
                        return quote! {
                            config.#field = Some(|| #err_resp.into_axum_html());
                        };
                    },
                    _ => {
                        // should be unreachable if validated properly.
                        return quote! {
                            config.#field = Some(|| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
                        };
                    }
                }
            }

            return quote! {
                config.#field = Some(|| g_server::Response::<()>::from((g_server::StatusCode::INTERNAL_SERVER_ERROR, (), ())).into_axum_empty());
            };
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
            CONFIG_FIELD_TIMEOUT_ERROR | CONFIG_FIELD_CONCURRENCY_LIMIT_ERROR => Self::validate_custom_errors(&value),

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
                Some("Text") | Some("String") | Some("text") | Some("string") => {},
                Some("Html") | Some("html") => {},
                _ => {
                    return Err(syn::Error::new(value.span(), "expected values: json(T), text(T), html(T), or ()"))
                }
            }
        } else {
            return Err(syn::Error::new(value.span(), "invalid custom errors values, expected values: json(T), text(T), html(T)"))
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
