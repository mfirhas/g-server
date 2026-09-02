use std::{collections::HashSet, fmt::Display};

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::format_ident;
use quote::quote;
use syn::{Expr, LitInt, LitStr, Result, Token, Type, braced, parse::ParseStream};

pub(crate) fn parse_server_body(input: ParseStream<'_>) -> Result<ServerBody> {
    let mut config = Vec::new();
    let mut context = None;
    let mut routes = Vec::new();

    while !input.is_empty() {
        let key: Ident = input.parse()?;

        match key.to_string().as_str() {
            // OPTIONAL.
            "config" => {
                input.parse::<Token![:]>()?;

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
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Get)?);
            }

            "post" => {
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Post)?);
            }

            "put" => {
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Put)?);
            }

            "patch" => {
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Patch)?);
            }

            "delete" => {
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Delete)?);
            }

            "options" => {
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Options)?);
            }

            "head" => {
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Head)?);
            }

            "trace" => {
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Trace)?);
            }

            "query" => {
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Query)?);
            }

            "any" => {
                input.parse::<Token![:]>()?;
                routes.push(crate::route::parse_route(input, HttpMethod::Any)?);
            }

            _ => {
                return Err(syn::Error::new(key.span(), "unexpected server member"));
            }
        }

        crate::consume_comma(input)?;
    }

    Ok(ServerBody {
        config,
        context,
        routes,
    })
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
    Mcp,
}

pub(crate) struct ServerBody {
    pub(crate) config: Vec<crate::config::ConfigEntry>,

    // OPTIONAL:
    // If omitted, context is ().
    pub(crate) context: Option<Type>,

    pub(crate) routes: Vec<crate::route::Route>,
}

pub(crate) struct GServer {
    pub(crate) servers: Vec<crate::server::Server>,
}

impl GServer {
    pub(crate) fn try_new(servers: Vec<Server>) -> Result<Self> {
        Self { servers }.validate()
    }

    pub(crate) fn validate(self) -> Result<Self> {
        validate_servers(&self.servers)?;
        Ok(self)
    }
}

fn validate_servers(servers: &[crate::server::Server]) -> Result<()> {
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

        if matches!(server.kind, crate::server::ServerKind::Http) {
            validate_routes(server)?;
        }
    }

    Ok(())
}

fn validate_routes(server: &crate::server::Server) -> Result<()> {
    let mut routes = HashSet::new();

    for route in &server.body.routes {
        if let Expr::Lit(expr) = &route.endpoint {
            if let syn::Lit::Str(endpoint) = &expr.lit {
                let endpoint_value = endpoint.value();

                let key = (route.method, endpoint_value.clone());

                if !routes.insert(key) {
                    return Err(syn::Error::new(
                        endpoint.span(),
                        format!("duplicate route: {} {}", route.method, endpoint_value,),
                    ));
                }
            }
        }
    }

    Ok(())
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

impl HttpMethod {
    pub(crate) fn method_tokens(&self) -> TokenStream2 {
        let ident = match self {
            crate::server::HttpMethod::Get => {
                format_ident!("Get")
            }

            crate::server::HttpMethod::Post => {
                format_ident!("Post")
            }

            crate::server::HttpMethod::Put => {
                format_ident!("Put")
            }

            crate::server::HttpMethod::Patch => {
                format_ident!("Patch")
            }

            crate::server::HttpMethod::Delete => {
                format_ident!("Delete")
            }

            crate::server::HttpMethod::Options => {
                format_ident!("Options")
            }

            crate::server::HttpMethod::Head => {
                format_ident!("Head")
            }

            crate::server::HttpMethod::Trace => {
                format_ident!("Trace")
            }

            crate::server::HttpMethod::Query => {
                format_ident!("Query")
            }

            crate::server::HttpMethod::Any => {
                format_ident!("Any")
            }
        };

        quote! {
            g_server::route::HttpMethod::#ident
        }
    }
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Options => write!(f, "OPTIONS"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Trace => write!(f, "TRACE"),
            HttpMethod::Query => write!(f, "QUERY"),
            HttpMethod::Any => write!(f, "ANY"),
        }
    }
}
