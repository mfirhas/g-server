use http::{HeaderMap, StatusCode};

#[derive(Debug, Clone)]
pub struct Response<Body = ()> {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Body,
}

pub trait IntoAxumStringResponse {
    fn into_string_response(self) -> axum::response::Response;
}

pub trait IntoAxumJsonResponse {
    fn into_json_response(self) -> axum::response::Response;
}

pub trait IntoAxumHtmlResponse {
    fn into_html_response(self) -> axum::response::Response;
}

use ::axum::response::IntoResponse;

impl IntoAxumStringResponse for Result<Response<String>, Response<String>> {
    fn into_string_response(self) -> axum::response::Response {
        match self {
            Ok(resp) => {
                let mut response = resp.body.into_response();

                *response.status_mut() = resp.status;
                response.headers_mut().extend(resp.headers);

                response
            }

            Err(resp) => {
                let mut response = resp.body.into_response();

                *response.status_mut() = resp.status;
                response.headers_mut().extend(resp.headers);

                response
            }
        }
    }
}

impl<ResB, ErrB> IntoAxumJsonResponse for Result<Response<ResB>, Response<ErrB>>
where
    ResB: ::serde::Serialize,
    ErrB: ::serde::Serialize,
{
    fn into_json_response(self) -> axum::response::Response {
        match self {
            Ok(resp) => {
                let mut response = axum::Json(resp.body).into_response();

                *response.status_mut() = resp.status;
                response.headers_mut().extend(resp.headers);

                response
            }

            Err(resp) => {
                let mut response = axum::Json(resp.body).into_response();

                *response.status_mut() = resp.status;
                response.headers_mut().extend(resp.headers);

                response
            }
        }
    }
}

impl IntoAxumHtmlResponse for Result<Response<String>, Response<String>> {
    fn into_html_response(self) -> axum::response::Response {
        match self {
            Ok(resp) => {
                let mut response = axum::response::Html(resp.body).into_response();

                *response.status_mut() = resp.status;
                response.headers_mut().extend(resp.headers);

                response
            }

            Err(resp) => {
                let mut response = axum::response::Html(resp.body).into_response();

                *response.status_mut() = resp.status;
                response.headers_mut().extend(resp.headers);

                response
            }
        }
    }
}
