use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Result;
use syn::Type;
use syn::parse::ParseStream;

pub(crate) fn parse_request_body(input: ParseStream<'_>) -> Result<RequestBody> {
    let kind: Ident = input.parse()?;

    match kind.to_string().as_str() {
        "String" => Ok(RequestBody::String),

        "Json" => {
            let body;
            syn::parenthesized!(body in input);

            let ty: Type = body.parse()?;

            Ok(RequestBody::Json(ty))
        }

        "Form" => {
            let body;
            syn::parenthesized!(body in input);

            let ty: Type = body.parse()?;

            Ok(RequestBody::Form(ty))
        }

        _ => Err(syn::Error::new(
            kind.span(),
            "expected `String`, `Json(Type)`, or `Form(Type)`",
        )),
    }
}

pub(crate) fn generate_body_extractor(body: &Option<RequestBody>) -> TokenStream2 {
    match body {
        // JSON body:
        //
        // request_body: Json(MyStruct)
        Some(RequestBody::Json(ty)) => {
            quote! {
                ::axum::extract::Json(body):
                    ::axum::extract::Json<#ty>,
            }
        }

        // Form body:
        //
        // request_body: Form(MyStruct)
        Some(RequestBody::Form(ty)) => {
            quote! {
                ::axum::extract::Form(body):
                    ::axum::extract::Form<#ty>,
            }
        }

        // String body:
        //
        // request_body: String
        Some(RequestBody::String) => {
            quote! {
                body: String,
            }
        }

        // No request_body:
        //
        // body is simply ().
        None => {
            quote! {
                body: (),
            }
        }
    }
}

pub(crate) enum RequestBody {
    // Json(StructType)
    Json(Type),

    // Form(StructType)
    Form(Type),

    // String
    String,
}
