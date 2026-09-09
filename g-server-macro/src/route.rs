use proc_macro2::{Ident, Span};
use quote::format_ident;
use syn::{Expr, Path, Result, Token, Type, braced, parse::ParseStream, spanned::Spanned};

use crate::server::{HttpMethod, Server};

pub(crate) fn parse_route(
    input: ParseStream<'_>,
    method: crate::server::HttpMethod,
) -> Result<Route> {
    let content;

    braced!(content in input);

    // OPTIONAL fields start with their defaults.
    let mut endpoint: Option<Expr> = None;
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

                handler = Some(parse_route_handler(&content)?);
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

    // sanitize endpoint
    let endpoint = crate::expr_to_string(&endpoint)
        .and_then(|ref expr_str| {
            syn::parse_str::<Expr>(&format!("\"{}\"", crate::sanitize_endpoint(expr_str))).ok()
        })
        .ok_or_else(|| syn::Error::new(endpoint.span(), "failed sanitizing route endpoint"))?;

    let handler = handler.unwrap_or_else(|| {
        RouteHandler::Path(syn::parse_quote! {
            g_server::route::unimplemented_handler
        })
    });

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
    let route = server.body.routes.get(index);

    if let Some(r) = route {
        match r.handler {
            RouteHandler::Path(ref path) => path.segments.last().map_or(
                format_ident!("__route_{}_{}", server.name.value(), index),
                |last| {
                    format_ident!(
                        "__route_{}_{}_{}",
                        server.name.value(),
                        last.ident.to_string(),
                        index
                    )
                },
            ),
            RouteHandler::Closure(_) => {
                let endpoint_str =
                    crate::expr_to_string(&r.endpoint).expect("endpoint must be a string");
                format_ident!(
                    "__route_{}_{}_{}",
                    server.name.value(),
                    endpoint_to_function_name_segment(&r.method, &endpoint_str),
                    index
                )
            }
        }
    } else {
        return format_ident!("__route_{}_{}", server.name.value(), index);
    }
}

fn endpoint_to_function_name_segment(verb: &HttpMethod, endpoint: &str) -> String {
    let endpoint_str: String = endpoint
        .chars()
        .map(|c| match c {
            '/' | '-' | '{' | '}' | ':' => '_',
            other => other,
        })
        .collect();

    format!("{}_{}", verb, endpoint_str)
}

#[derive(Clone)]
pub(crate) struct Route {
    pub(crate) method: crate::server::HttpMethod,

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
    pub(crate) handler: RouteHandler,

    // OPTIONAL.
    // Defaults to Json.
    pub(crate) response_body: crate::response_body::ResponseBody,
}

#[derive(Clone)]
pub(crate) enum RouteHandler {
    Path(syn::Path),
    Closure(syn::ExprClosure),
}

fn parse_route_handler(input: ParseStream<'_>) -> Result<RouteHandler> {
    let expr: syn::Expr = input.parse()?;

    let handler = match expr {
        syn::Expr::Path(expr) => RouteHandler::Path(expr.path),
        syn::Expr::Closure(expr) => RouteHandler::Closure(expr),
        expr => {
            return Err(syn::Error::new_spanned(
                expr,
                "expected a handler path or closure",
            ));
        }
    };

    Ok(handler)
}
