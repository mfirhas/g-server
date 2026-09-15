use std::fmt::Display;

use proc_macro2::Ident;
use syn::parse::ParseStream;
use syn::{Result, Type};

pub(crate) fn parse_response_body(input: ParseStream<'_>) -> Result<ResponseBody> {
    let ident: Ident = input.parse()?;

    match ident.to_string().as_str() {
        "Json" | "json" => {
            let body;
            syn::parenthesized!(body in input);

            let ty: Type = body.parse()?;

            Ok(ResponseBody::Json(ty))
        }

        "String" | "Text" | "string" | "text" => Ok(ResponseBody::String),

        "Html" | "html" => Ok(ResponseBody::Html),

        "Empty" | "empty" | "None" | "none" => Ok(ResponseBody::Empty),

        _ => Err(syn::Error::new(
            ident.span(),
            "supported response body type: json, string/text, html",
        )),
    }
}

#[derive(Clone)]
pub(crate) enum ResponseBody {
    Json(Type),
    String,
    Html,
    Empty,
}

impl Default for ResponseBody {
    fn default() -> Self {
        ResponseBody::Json(syn::parse_quote!(()))
    }
}

impl Display for ResponseBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(_) => write!(f, "Json"),
            Self::String => write!(f, "String"),
            Self::Html => write!(f, "Html"),
            Self::Empty => write!(f, "Empty"),
        }
    }
}
