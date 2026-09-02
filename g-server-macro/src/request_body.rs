use proc_macro2::Ident;
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

pub(crate) enum RequestBody {
    // Json(StructType)
    Json(Type),

    // Form(StructType)
    Form(Type),

    // String
    String,
}
