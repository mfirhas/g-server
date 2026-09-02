use http::{HeaderMap, StatusCode};

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

    pub fn with_text(mut self, body: String) -> Response<String> {
        self.headers.insert(
            crate::http::header::CONTENT_TYPE,
            crate::http::HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        Response {
            status: self.status,
            headers: self.headers,
            body,
        }
    }

    pub fn with_html(mut self, body: String) -> Response<String> {
        self.headers.insert(
            crate::http::header::CONTENT_TYPE,
            crate::http::HeaderValue::from_static("text/html; charset=utf-8"),
        );
        Response {
            status: self.status,
            headers: self.headers,
            body,
        }
    }

    pub fn with_json<Json>(mut self, body: Json) -> Response<Json>
    where
        Json: ::serde::Serialize,
    {
        self.headers.insert(
            crate::http::header::CONTENT_TYPE,
            crate::http::HeaderValue::from_static("application/json"),
        );
        Response {
            status: self.status,
            headers: self.headers,
            body,
        }
    }
}

use ::axum::response::{IntoResponse, Response as AxumResponse};

impl Response {
    pub fn into_axum_empty(self) -> AxumResponse {
        let mut resp = ().into_response();

        *resp.status_mut() = self.status;

        resp.headers_mut().extend(self.headers);

        resp
    }
}

impl Response<String> {
    pub fn into_axum_string(self) -> AxumResponse {
        let mut resp = self.body.into_response();

        *resp.status_mut() = self.status;

        resp.headers_mut().extend(self.headers);

        resp
    }

    pub fn into_axum_html(self) -> AxumResponse {
        let mut resp = ::axum::response::Html(self.body).into_response();

        *resp.status_mut() = self.status;

        resp.headers_mut().extend(self.headers);

        resp
    }
}

impl<Json> Response<Json>
where
    Json: ::serde::Serialize,
{
    pub fn into_axum_json(self) -> AxumResponse {
        let mut resp = ::axum::response::Json(self.body).into_response();

        *resp.status_mut() = self.status;

        resp.headers_mut().extend(self.headers);

        resp
    }
}
