use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::request_body::RequestBody;

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
