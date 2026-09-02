use proc_macro2::{Ident, Span};
use quote::format_ident;
use syn::{Expr, Path, Result, Token, Type, braced, parse::ParseStream};

use crate::Server;

pub(crate) fn parse_route(input: ParseStream<'_>, method: crate::HttpMethod) -> Result<Route> {
    let content;

    braced!(content in input);

    // OPTIONAL fields start with their defaults.
    let mut endpoint = None;
    let mut config = Vec::new();
    let mut path_params = None;
    let mut query_params = None;
    let mut request_body = None;
    let mut middlewares = Vec::new();

    // MANDATORY, therefore remains None until parsed.
    let mut handler = None;

    // OPTIONAL, defaults to Json.
    let mut response_body = crate::response_body::ResponseBody::default();

    while !content.is_empty() {
        let key: Ident = content.parse()?;

        match key.to_string().as_str() {
            // MANDATORY.
            "endpoint" => {
                content.parse::<Token![:]>()?;

                endpoint = Some(content.parse()?);
            }

            // OPTIONAL.
            "config" => {
                content.parse::<Token![:]>()?;

                let body;

                braced!(body in content);

                config = crate::config::parse_config(&body)?;
            }

            // OPTIONAL.
            //
            // Omitted => Path<()>.
            "path_params" => {
                content.parse::<Token![:]>()?;

                path_params = Some(content.parse()?);
            }

            // OPTIONAL.
            //
            // Omitted => Query<()>.
            "query_params" => {
                content.parse::<Token![:]>()?;

                query_params = Some(content.parse()?);
            }

            // OPTIONAL.
            //
            // Supported:
            //
            // request_body: String
            // request_body: Json(MyStruct)
            // request_body: Form(MyStruct)
            "request_body" => {
                content.parse::<Token![:]>()?;

                request_body = Some(crate::request_body::parse_request_body(&content)?);
            }

            // OPTIONAL.
            "middlewares" => {
                content.parse::<Token![:]>()?;

                let body;

                syn::bracketed!(body in content);

                while !body.is_empty() {
                    middlewares.push(body.parse()?);

                    crate::consume_comma(&body)?;
                }
            }

            // MANDATORY.
            "handler" => {
                content.parse::<Token![:]>()?;

                handler = Some(content.parse()?);
            }

            // OPTIONAL.
            //
            // Default = Json.
            "response_body" => {
                content.parse::<Token![:]>()?;

                response_body = crate::response_body::parse_response_body(&content)
                    .map_err(|err| syn::Error::new(key.span(), err.to_string()))?;
            }

            _ => {
                return Err(syn::Error::new(key.span(), "unexpected route member"));
            }
        }

        crate::consume_comma(&content)?;
    }

    // --------------------------------------------------------
    // Mandatory validation
    // --------------------------------------------------------

    let endpoint = endpoint.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "route requires mandatory field `endpoint`",
        )
    })?;

    let handler = handler.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "route requires mandatory field `handler`",
        )
    })?;

    Ok(Route {
        method,
        endpoint,
        config,
        path_params,
        query_params,
        request_body,
        middlewares,
        handler,
        response_body,
    })
}

pub(crate) fn route_function_ident(server: &Server, index: usize) -> Ident {
    let handler_name = server
        .body
        .routes
        .get(index)
        .and_then(|route| route.handler.segments.last())
        .map(|segment| segment.ident.to_string())
        .unwrap_or_else(|| format!("route_{index}"));

    format_ident!("__route_{}_{}", server.name.value(), handler_name)
}

pub(crate) struct Route {
    pub(crate) method: crate::HttpMethod,

    // MANDATORY.
    //
    // Kept as an expression so this can eventually support:
    //
    // endpoint: "/foo",
    // endpoint: SOME_STATIC,
    //
    // instead of only a string literal.
    pub(crate) endpoint: Expr,

    // OPTIONAL.
    pub(crate) config: Vec<crate::config::ConfigEntry>,

    // OPTIONAL.
    // If omitted => Path<()>.
    pub(crate) path_params: Option<Type>,

    // OPTIONAL.
    // If omitted => Query<()>.
    pub(crate) query_params: Option<Type>,

    // OPTIONAL.
    // If omitted => body ().
    pub(crate) request_body: Option<crate::request_body::RequestBody>,

    // OPTIONAL.
    pub(crate) middlewares: Vec<Path>,

    // MANDATORY.
    pub(crate) handler: Path,

    // OPTIONAL.
    // Defaults to Json.
    pub(crate) response_body: crate::response_body::ResponseBody,
}
