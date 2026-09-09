#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::Ident;
use rand::{RngExt, distr::Alphanumeric};
use syn::{
    Expr, LitInt, LitStr, Result, Token, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

mod config;
mod group;
mod request_body;
mod response_body;
mod route;
mod server;
use server::GServer;

mod axum_impl;

#[proc_macro]
pub fn gserver(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as GServer);

    match crate::axum_impl::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
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
                "http" => crate::server::ServerKind::Http,
                "mcp" => crate::server::ServerKind::Mcp,

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

            let body = crate::server::parse_server_body(&body)?;

            consume_comma(input)?;

            servers.push(crate::server::Server {
                kind,
                name,
                ip,
                port,
                body,
            });
        }

        Self::try_new(servers)
    }
}

pub(crate) fn consume_comma(input: ParseStream<'_>) -> Result<()> {
    if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
    }

    Ok(())
}

pub(crate) fn expr_to_string(expr: &Expr) -> Option<String> {
    if let Expr::Lit(expr) = &expr {
        if let syn::Lit::Str(prefix) = &expr.lit {
            Some(prefix.value())
        } else {
            None
        }
    } else {
        None
    }
}

pub(crate) fn random_6_chars() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect()
}

/// Sanitizes an HTTP route endpoint into a normalized path representation.
///
/// Axum performs exact path matching by default, which means visually
/// equivalent endpoints such as `users`, `/users`, and `/users/` can be
/// treated differently. This function normalizes common endpoint patterns
/// before the endpoint is passed to Axum.
///
/// The following transformations are performed:
///
/// - Adds a leading `/` when the endpoint does not have one.
/// - Collapses consecutive `/` characters into a single `/`.
/// - Removes trailing `/` characters, except when the endpoint is `/`.
/// - Preserves route parameters and other non-slash characters unchanged.
///
/// # Examples
///
/// ```text
/// ""             -> "/"
/// "users"        -> "/users"
/// "/users"       -> "/users"
/// "/users/"      -> "/users"
/// "/users///"    -> "/users"
/// "/users//:id"  -> "/users/:id"
/// "//users/:id"  -> "/users/:id"
/// "/"            -> "/"
/// ```
///
/// # Route Parameters
///
/// Route parameters are preserved as-is. For example:
///
/// ```text
/// "/users/:id/"      -> "/users/:id"
/// "/users/:id/posts" -> "/users/:id/posts"
/// ```
///
/// The function does not attempt to interpret or rewrite route syntax such
/// as `:id`, wildcards, or other Axum-specific patterns.
///
/// # Non-Goals
///
/// This function does not:
///
/// - URL-decode or URL-encode the endpoint.
/// - Normalize `.` or `..` path segments.
/// - Rewrite route parameters.
/// - Normalize query strings.
/// - Validate whether the resulting endpoint is a valid Axum route.
///
/// It is intended only to perform basic path normalization, while leaving
/// the actual route syntax and validation to Axum.
///
/// # Panics
///
/// This function never panics due to the endpoint contents.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(sanitize_endpoint("users"), "/users");
/// assert_eq!(sanitize_endpoint("/users/"), "/users");
/// assert_eq!(sanitize_endpoint("/users//:id"), "/users/:id");
/// assert_eq!(sanitize_endpoint("/"), "/");
/// ```
pub(crate) fn sanitize_endpoint(endpoint: &str) -> String {
    let endpoint = endpoint.trim();

    if endpoint.is_empty() {
        return "/".into();
    }

    // Always make the endpoint absolute.
    let endpoint = if endpoint.starts_with('/') {
        endpoint.to_owned()
    } else {
        format!("/{endpoint}")
    };

    // Collapse consecutive slashes.
    let mut sanitized = String::with_capacity(endpoint.len());
    let mut previous_was_slash = false;

    for c in endpoint.chars() {
        if c == '/' {
            if !previous_was_slash {
                sanitized.push(c);
            }

            previous_was_slash = true;
        } else {
            sanitized.push(c);
            previous_was_slash = false;
        }
    }

    // Normalize trailing slash, except for `/`.
    if sanitized.len() > 1 {
        sanitized.truncate(sanitized.trim_end_matches('/').len());
    }

    sanitized
}
