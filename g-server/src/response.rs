use std::fmt::Display;

use crate::http::{HeaderMap, StatusCode};

pub type Result<T, E> = std::result::Result<Response<T>, Response<E>>;

const HEADER_CONTENT_TYPE_TEXT_PLAIN: crate::http::HeaderValue =
    crate::http::HeaderValue::from_static("text/plain; charset=utf-8");

const HEADER_CONTENT_TYPE_TEXT_HTML: crate::http::HeaderValue =
    crate::http::HeaderValue::from_static("text/html; charset=utf-8");

const HEADER_CONTENT_TYPE_APP_JSON: crate::http::HeaderValue =
    crate::http::HeaderValue::from_static("application/json");

#[derive(Debug, Clone)]
pub struct Response<Body = ()> {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Body,
}

impl Response {
    pub fn new() -> Self {
        Self {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: (),
        }
    }

    pub fn with_status(mut self, status_code: StatusCode) -> Self {
        self.status = status_code;
        self
    }

    pub fn with_header(mut self, header: HeaderMap) -> Self {
        self.headers = header;
        self
    }

    pub fn with_text<S>(self, body: S) -> Response<S>
    where
        S: Display,
    {
        Response {
            status: self.status,
            headers: self.headers,
            body,
        }
    }

    pub fn with_html<S>(self, body: S) -> Response<S>
    where
        S: Display,
    {
        Response {
            status: self.status,
            headers: self.headers,
            body,
        }
    }

    pub fn with_json<Json>(self, body: Json) -> Response<Json>
    where
        Json: ::serde::Serialize,
    {
        Response {
            status: self.status,
            headers: self.headers,
            body,
        }
    }
}

use crate::axum::response::{IntoResponse, Response as AxumResponse};

impl<T> Response<T> {
    pub fn into_axum_empty(self) -> AxumResponse {
        let mut resp = ().into_response();

        *resp.status_mut() = self.status;

        resp.headers_mut().extend(self.headers);

        resp
    }
}

impl<S> Response<S>
where
    S: Display,
{
    pub fn into_axum_string(self) -> AxumResponse {
        let mut resp = self.body.to_string().into_response();

        *resp.status_mut() = self.status;

        resp.headers_mut().extend(self.headers);

        resp.headers_mut()
            .entry(crate::http::header::CONTENT_TYPE)
            .or_insert(HEADER_CONTENT_TYPE_TEXT_PLAIN);

        resp
    }

    pub fn into_axum_html(self) -> AxumResponse {
        let mut resp = crate::axum::response::Html(self.body.to_string()).into_response();

        *resp.status_mut() = self.status;

        resp.headers_mut().extend(self.headers);

        resp.headers_mut()
            .entry(crate::http::header::CONTENT_TYPE)
            .or_insert(HEADER_CONTENT_TYPE_TEXT_HTML);

        resp
    }
}

impl<Json> Response<Json>
where
    Json: ::serde::Serialize,
{
    pub fn into_axum_json(self) -> AxumResponse {
        let mut resp = crate::axum::response::Json(self.body).into_response();

        *resp.status_mut() = self.status;

        *resp.headers_mut() = self.headers;

        resp.headers_mut()
            .entry(crate::http::header::CONTENT_TYPE)
            .or_insert(HEADER_CONTENT_TYPE_APP_JSON);

        resp
    }
}

impl<T> From<(StatusCode, HeaderMap, T)> for Response<T> {
    #[inline]
    fn from((status, headers, body): (StatusCode, HeaderMap, T)) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }
}

impl<T> From<(StatusCode, (), T)> for Response<T> {
    #[inline]
    fn from((status, _, body): (StatusCode, (), T)) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body,
        }
    }
}

impl<T> From<(StatusCode, T)> for Response<T> {
    #[inline]
    fn from((status, body): (StatusCode, T)) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body,
        }
    }
}

impl From<StatusCode> for Response {
    #[inline]
    fn from(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: (),
        }
    }
}

pub trait ErrorMessage: Sized {
    fn err_msg(self, err: &str) -> Self;
}

impl<E> ErrorMessage for Response<E>
where
    E: ErrorMessage,
{
    fn err_msg(self, err: &str) -> Self {
        Self {
            status: self.status,
            headers: self.headers,
            body: self.body.err_msg(err),
        }
    }
}

impl ErrorMessage for String {
    fn err_msg(self, _: &str) -> Self {
        self
    }
}

impl ErrorMessage for &'static str {
    fn err_msg(self, _: &str) -> Self {
        self
    }
}
