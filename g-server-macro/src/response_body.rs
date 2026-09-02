use std::{fmt::Display, str::FromStr};

use proc_macro2::{Ident, Span};
use syn::Result;
use syn::parse::ParseStream;

pub(crate) fn parse_response_body(input: ParseStream<'_>) -> Result<ResponseBody> {
    let ident: Ident = input.parse()?;

    crate::response_body::ResponseBody::from_str(ident.to_string().as_str())
        .map_err(|err| syn::Error::new(Span::call_site(), err))
}

#[derive(Clone, Copy, Default)]
pub(crate) enum ResponseBody {
    #[default]
    Json,
    String,
    Html,
    Empty,
}

impl Display for ResponseBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "Json"),
            Self::String => write!(f, "String"),
            Self::Html => write!(f, "Html"),
            Self::Empty => write!(f, "Empty"),
        }
    }
}

impl FromStr for ResponseBody {
    type Err = &'static str;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Json" | "json" => Ok(ResponseBody::Json),

            "String" | "Text" | "string" | "text" => Ok(ResponseBody::String),

            "Html" | "html" => Ok(ResponseBody::Html),

            "Empty" | "empty" | "None" | "none" => Ok(ResponseBody::Empty),

            _ => {
                return Err("supported response body type: json, string/text, html");
            }
        }
    }
}
