use std::{collections::HashSet, fmt::Display};

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::format_ident;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Expr, LitInt, LitStr, Result, Token, Type, braced, parse::ParseStream};

use crate::config::ConfigEntry;
use crate::group::Group;
use crate::group::GroupMember;
use crate::route::Route;

pub(crate) fn parse_server_body(input: ParseStream<'_>) -> Result<ServerBody> {
    let mut config = Vec::new();
    let mut context = None;
    let mut routes = Vec::new();
    let mut groups = Vec::new();

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
                input.parse::<Token![:]>()?;
                groups.push(crate::group::parse_group(input)?);
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
        groups,
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

    pub(crate) groups: Vec<crate::group::Group>,
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

        validate_http_server_configs(server)?;

        validate_http_routes(server)?;

        validate_http_group_routes(server)?;
    }

    Ok(())
}

fn validate_http_server_configs(server: &crate::server::Server) -> Result<()> {
    if let ServerKind::Http = server.kind {
        let server_body = &server.body;

        for route in server_body.routes.iter() {
            validate_non_global_configs(&route.config)?;
        }

        for group in server_body.groups.iter() {
            validate_group_configs(group)?;
        }
    }
    Ok(())
}

fn validate_group_configs(group: &crate::group::Group) -> Result<()> {
    validate_non_global_configs(&group.config)?;

    for member in group.members.iter() {
        match member {
            GroupMember::Route(route) => validate_non_global_configs(&route.config)?,
            GroupMember::Group(group) => validate_group_configs(group)?,
        }
    }

    Ok(())
}

// configs that only exist at global config
static GLOBAL_CONFIG: [&str; 1] = ["normalize_endpoint"];

fn validate_non_global_configs(configs: &[ConfigEntry]) -> Result<()> {
    for cfg in configs {
        let ident_name = cfg.name.to_string();
        if GLOBAL_CONFIG.contains(&ident_name.as_str()) {
            return Err(syn::Error::new(
                cfg.name.span(),
                format!("`{}` only allowed for global config", &ident_name),
            ));
        }
    }

    Ok(())
}

fn validate_http_routes(server: &crate::server::Server) -> Result<()> {
    if let ServerKind::Http = server.kind {
        let mut routes = HashSet::new();

        for route in &server.body.routes {
            if let Expr::Lit(expr) = &route.endpoint {
                if let syn::Lit::Str(endpoint) = &expr.lit {
                    let endpoint_value = endpoint.value();

                    validate_endpoint(&endpoint_value)
                        .map_err(|err| syn::Error::new(endpoint.span(), err))?;

                    let normalized_endpoint = normalize_route_endpoint(&endpoint_value);

                    if let HttpMethod::Any = route.method {
                        if routes
                            .iter()
                            .any(|(_, endpoint)| endpoint == &normalized_endpoint)
                        {
                            return Err(syn::Error::new(
                                endpoint.span(),
                                format!("duplicate endpoint for `any`: {}", &endpoint_value),
                            ));
                        }
                    } else if routes.iter().any(|(method, endpoint)| {
                        matches!(method, HttpMethod::Any) && endpoint == &normalized_endpoint
                    }) {
                        return Err(syn::Error::new(
                            endpoint.span(),
                            format!("duplicate endpoint for existing `any`: {}", &endpoint_value),
                        ));
                    }

                    let key = (route.method, normalized_endpoint.clone());

                    if !routes.insert(key) {
                        return Err(syn::Error::new(
                            endpoint.span(),
                            format!("duplicate route: {} {}", route.method, normalized_endpoint,),
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_http_group_routes(server: &crate::server::Server) -> Result<()> {
    if let ServerKind::Http = server.kind {
        let mut routes_set = HashSet::new();

        for group in &server.body.groups {
            let routes = flatten_group_endpoints(group);
            for route in &routes {
                validate_endpoint(&route.0)
                    .map_err(|err| syn::Error::new(group.prefix.span(), err))?;

                let normalized_endpoint = normalize_route_endpoint(route.0.as_str());

                if let HttpMethod::Any = route.1.method {
                    if routes_set
                        .iter()
                        .any(|(_, endpoint)| endpoint == &normalized_endpoint)
                    {
                        return Err(syn::Error::new(
                            route.1.endpoint.span(),
                            format!("duplicate endpoint for `any`: {}", &route.0),
                        ));
                    }
                } else if routes_set.iter().any(|(method, endpoint)| {
                    matches!(method, HttpMethod::Any) && endpoint == &normalized_endpoint
                }) {
                    return Err(syn::Error::new(
                        route.1.endpoint.span(),
                        format!("duplicate endpoint for existing `any`: {}", &route.0),
                    ));
                }

                let key = (route.1.method, normalized_endpoint.clone());

                if !routes_set.insert(key) {
                    return Err(syn::Error::new(
                        group.prefix.span(),
                        format!(
                            "duplicate group route: {} {}",
                            route.1.method, normalized_endpoint
                        ),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn normalize_route_endpoint(endpoint: &str) -> String {
    endpoint
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "{...}".to_owned()
            } else {
                segment.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Validates that an endpoint path template has properly balanced,
/// non-empty, non-nested `{param}` segments, and that each parameter
/// occupies its own full path segment (e.g. `/v1/user{id}` is invalid —
/// it must be `/v1/user/{id}`).
pub fn validate_endpoint(path: &str) -> std::result::Result<(), &'static str> {
    let mut open = false;
    let mut last_open_idx = 0;

    for (i, ch) in path.char_indices() {
        match ch {
            '{' => {
                if open {
                    return Err("nested '{' found before previous one was closed");
                }
                open = true;
                last_open_idx = i;
            }
            '}' => {
                if !open {
                    return Err("unmatched '}' with no preceding '{'");
                }
                if i == last_open_idx + 1 {
                    return Err("empty parameter '{}' found");
                }
                open = false;
            }
            _ => {}
        }
    }

    if open {
        return Err("unclosed '{' found in path");
    }

    for segment in path.split('/') {
        let has_open = segment.contains('{');
        let has_close = segment.contains('}');

        if has_open || has_close {
            let starts_right = segment.starts_with('{');
            let ends_right = segment.ends_with('}');
            let only_one_each =
                segment.matches('{').count() == 1 && segment.matches('}').count() == 1;

            if !(starts_right && ends_right && only_one_each) {
                return Err(
                    "parameter must occupy its entire path segment, e.g. '/v1/{id}' not '/v1/user{id}'",
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod validate_endpoint_tests {
    use super::*;

    #[test]
    fn valid_paths() {
        assert!(validate_endpoint("/v1/user/{user_id}").is_ok());
        assert!(validate_endpoint("/v1/user/{user_id}/posts/{post_id}").is_ok());
        assert!(validate_endpoint("/v1/health").is_ok());
    }

    #[test]
    fn missing_closing_brace() {
        assert_eq!(
            validate_endpoint("/v1/user/{user_id"),
            Err("unclosed '{' found in path")
        );
    }

    #[test]
    fn missing_opening_brace() {
        assert_eq!(
            validate_endpoint("/v1/user/user_id}"),
            Err("unmatched '}' with no preceding '{'")
        );
    }

    #[test]
    fn empty_param() {
        assert_eq!(
            validate_endpoint("/v1/user/{}"),
            Err("empty parameter '{}' found")
        );
    }

    #[test]
    fn nested_brace() {
        assert_eq!(
            validate_endpoint("/v1/user/{user_{id}"),
            Err("nested '{' found before previous one was closed")
        );
    }

    #[test]
    fn param_sharing_segment_with_literal() {
        assert_eq!(
            validate_endpoint("/v1/user{id}"),
            Err(
                "parameter must occupy its entire path segment, e.g. '/v1/{id}' not '/v1/user{id}'"
            )
        );
        assert_eq!(
            validate_endpoint("/v1/{id}suffix"),
            Err(
                "parameter must occupy its entire path segment, e.g. '/v1/{id}' not '/v1/user{id}'"
            )
        );
        assert_eq!(
            validate_endpoint("/v1/a{id}b"),
            Err(
                "parameter must occupy its entire path segment, e.g. '/v1/{id}' not '/v1/user{id}'"
            )
        );
    }
}

// group validation
fn flatten_group_endpoints(group: &Group) -> Vec<(String, &Route)> {
    fn visit<'a>(group: &'a Group, parent_prefix: &str, endpoints: &mut Vec<(String, &'a Route)>) {
        let prefix = join(parent_prefix, expr_string(&group.prefix));

        for member in &group.members {
            match member {
                GroupMember::Route(route) => {
                    let endpoint = join(&prefix, expr_string(&route.endpoint));
                    endpoints.push((endpoint, route));
                }

                GroupMember::Group(group) => {
                    visit(group, &prefix, endpoints);
                }
            }
        }
    }

    fn expr_string(expr: &Expr) -> String {
        match expr {
            Expr::Lit(expr) => match &expr.lit {
                syn::Lit::Str(value) => value.value(),
                _ => panic!("expected string literal"),
            },
            _ => panic!("expected string literal"),
        }
    }

    fn join(parent: &str, child: String) -> String {
        if parent.is_empty() {
            return child;
        }

        if child.is_empty() {
            return parent.to_owned();
        }

        format!(
            "{}/{}",
            parent.trim_end_matches('/'),
            child.trim_start_matches('/'),
        )
    }

    let mut endpoints = Vec::new();
    visit(group, "", &mut endpoints);
    endpoints
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
