#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::Ident;
use syn::{
    LitInt, LitStr, Result, Token, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

mod config;
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
