use std::{fmt::Display, str::FromStr};

#[derive(Clone, Copy, Default)]
pub(crate) enum ResponseBody {
    #[default]
    Json,
    String,
    Html,
}

impl Display for ResponseBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "Json"),
            Self::String => write!(f, "String"),
            Self::Html => write!(f, "Html"),
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

            _ => {
                return Err("supported response body type: json, string/text, html");
            }
        }
    }
}
