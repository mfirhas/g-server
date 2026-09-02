use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use std::{fmt::Display, str::FromStr};
use syn::{Expr, Result, Token, parse::ParseStream, spanned::Spanned};

/// Parses config.
///
/// ```
/// gserver! {
///     ...
///     config: {
///         name: value, // parses this
///     }
///     ...
///     get: {
///         config: {
///             name: value, // and this
///         }
///     }
/// }
/// ```
///
/// Each config entry will be parsed into [`ConfigEntry`].
pub(crate) fn parse_config(input: ParseStream<'_>) -> Result<Vec<ConfigEntry>> {
    let mut entries = Vec::new();

    while !input.is_empty() {
        let name: Ident = input.parse()?;

        input.parse::<Token![:]>()?;

        let value: Expr = input.parse()?;

        let config = ConfigEntry::try_new(name.clone(), value)
            .map_err(|err| syn::Error::new(name.span(), err.to_string()))?;

        entries.push(config);

        crate::consume_comma(input)?;
    }

    Ok(entries)
}

/// Generation function for global config.
pub(crate) fn generate_global_config(entries: &[ConfigEntry]) -> TokenStream2 {
    let assignments = entries.iter().map(|entry| {
        let field = &entry.name;

        let value = &entry.value;

        quote! {
            global_config.#field = (#value).into();
        }
    });

    quote! {
        let mut global_config = g_server::Config::default();
        #(#assignments)*
    }
}

/// Generation function for route config.
pub(crate) fn generate_route_config(entries: &[ConfigEntry]) -> TokenStream2 {
    let assignments = entries.iter().map(|entry| {
        let field = &entry.name;

        let value = &entry.value;

        quote! {
            config.#field = (#value).into();
        }
    });

    quote! {
        let mut config = g_server::Config::empty();
        #(#assignments)*
    }
}

/// Represents config entry.
///
/// ```
/// gserver! {
///     ...
///     config: {
///         name: value,
///         ...
///     }
///     ...
///
///     get: {
///         config: {
///             name: value,
///             ...
///         },
///         ...
///     }
/// }
/// ```
///
/// `name` is from supported configs, from [`Compression`].
///
/// `value` is from supported value for each config entry.
///
/// This config entry will be mapped into `g_server::Config`.
pub(crate) struct ConfigEntry {
    pub(crate) name: Ident,
    pub(crate) value: Expr,
}

impl ConfigEntry {
    pub(crate) fn try_new(name: Ident, mut value: Expr) -> Result<Self> {
        match name.to_string().as_str() {
            "timeout" => Self::validate_integer(&value),
            "concurrency_limit" => Self::validate_integer(&value),
            "body_limit" => Self::validate_integer(&value),
            "compression" => Self::validate_compression(&mut value),

            _ => Err(syn::Error::new(
                name.span(),
                format!("unknown config `{}`", name),
            )),
        }?;

        Ok(Self { name, value })
    }

    fn validate_integer(value: &Expr) -> Result<()> {
        match value {
            Expr::Lit(expr) if matches!(&expr.lit, syn::Lit::Int(_)) => Ok(()),

            _ => Err(syn::Error::new(value.span(), "expects an integer")),
        }
    }

    fn validate_compression(value: &mut Expr) -> Result<()> {
        let value_str = value.to_token_stream().to_string();
        let c = Compression::from_str(value_str.as_str())
            .map_err(|err| syn::Error::new(Span::call_site(), err))?;
        let c_ident = syn::Ident::new(c.to_string().as_str(), Span::call_site());
        *value = syn::parse2(quote! { g_server::Compression::#c_ident })?;

        Ok(())
    }
}

// COMPRESSION config
// --------------------------------------------------------------

#[derive(Clone, Copy)]
pub(crate) enum Compression {
    Deflate,
    Gzip,
    Brotli,
    Zstd,
    All,
}

impl Compression {
    pub(crate) fn display_list() -> String {
        String::from("deflate, gzip, brotli, zstd, all")
    }
}

impl Display for Compression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deflate => write!(f, "Deflate"),
            Self::Gzip => write!(f, "Gzip"),
            Self::Brotli => write!(f, "Brotli"),
            Self::Zstd => write!(f, "Zstd"),
            Self::All => write!(f, "All"),
        }
    }
}

impl FromStr for Compression {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Deflate" | "deflate" => Ok(Self::Deflate),
            "Gzip" | "gzip" => Ok(Self::Gzip),
            "Brotli" | "brotli" => Ok(Self::Brotli),
            "Zstd" | "zstd" => Ok(Self::Zstd),
            "All" | "all" => Ok(Self::All),
            other => Err(format!(
                "`{}` is invalid/unsupported compression method, supported: {}",
                other,
                Self::display_list()
            )),
        }
    }
}
