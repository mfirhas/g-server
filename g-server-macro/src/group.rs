use proc_macro2::{Ident, Span};
use quote::format_ident;
use syn::{Expr, Path, Result, Token, braced, parse::ParseStream};

use crate::{
    route::Route,
    server::{HttpMethod, Server},
};

pub(crate) fn parse_group(input: ParseStream<'_>) -> Result<Group> {
    let content;

    braced!(content in input);

    let mut prefix = None;
    let mut config = Vec::new();
    let mut middlewares = Vec::new();
    let mut members = Vec::new();

    while !content.is_empty() {
        let key: Ident = content.parse()?;

        match key.to_string().as_str() {
            "prefix" => {
                content.parse::<Token![:]>()?;
                prefix = Some(content.parse()?);
            }

            "config" => {
                content.parse::<Token![:]>()?;

                let body;

                braced!(body in content);

                config = crate::config::parse_config(&body)?;
            }

            "middlewares" => {
                content.parse::<Token![:]>()?;

                let body;

                syn::bracketed!(body in content);

                while !body.is_empty() {
                    middlewares.push(body.parse()?);

                    crate::consume_comma(&body)?;
                }
            }

            "members" => {
                content.parse::<Token![:]>()?;

                let body;

                syn::bracketed!(body in content);

                members = parse_group_members(&body)?;
            }

            _ => {
                return Err(syn::Error::new(key.span(), "unexpected group key"));
            }
        }

        crate::consume_comma(&content)?;
    }

    let prefix = prefix.ok_or_else(|| {
        syn::Error::new(Span::call_site(), "group requires mandatory field `prefix`")
    })?;

    if members.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "group must have atleast 1 member",
        ));
    }

    Ok(Group {
        prefix,
        config,
        middlewares,
        members,
    })
}

fn parse_group_members(input: ParseStream<'_>) -> Result<Vec<GroupMember>> {
    let mut group_members = Vec::new();

    while !input.is_empty() {
        let key: Ident = input.parse()?;

        match key.to_string().as_str() {
            // nested group
            "group" => {
                input.parse::<Token![:]>()?;
                let group = parse_group(input)?;
                group_members.push(GroupMember::Group(Box::new(group)));
            }

            // Routes.
            "get" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Get)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            "post" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Post)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            "put" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Put)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            "patch" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Patch)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            "delete" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Delete)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            "options" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Options)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            "head" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Head)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            "trace" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Trace)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            "query" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Query)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            "any" => {
                input.parse::<Token![:]>()?;
                let route = crate::route::parse_route(input, HttpMethod::Any)?;
                group_members.push(GroupMember::Route(Box::new(route)));
            }

            _ => {
                return Err(syn::Error::new(key.span(), "unexpected group member key"));
            }
        }

        crate::consume_comma(&input)?;
    }

    Ok(group_members)
}

pub(crate) fn group_function_ident(server: &Server, index: usize) -> Ident {
    let group_name = server
        .body
        .groups
        .get(index)
        .and_then(|group| Some(group.prefix.clone()))
        .and_then(|expr| crate::expr_to_string(&expr))
        .map(|prefix| sanitize_prefix(prefix.as_str()))
        .unwrap_or_else(|| format!("group_{index}"));

    format_ident!("__group_{}_{}", server.name.value(), group_name)
}

// remove leading `/`, change other occurrences of `/` and `-` as `_`
pub(crate) fn sanitize_prefix(prefix: &str) -> String {
    prefix
        .strip_prefix('/')
        .unwrap_or(prefix)
        .chars()
        .map(|c| match c {
            '/' | '-' => '_',
            other => other,
        })
        .collect()
}

#[derive(Clone)]
pub(crate) struct Group {
    // mandatory
    pub(crate) prefix: Expr,

    // optional
    pub(crate) config: Vec<crate::config::ConfigEntry>,

    // optional
    pub(crate) middlewares: Vec<Path>,

    // mandatory
    pub(crate) members: Vec<GroupMember>,
}

#[derive(Clone)]
pub(crate) enum GroupMember {
    Route(Box<Route>),
    Group(Box<Group>),
}
