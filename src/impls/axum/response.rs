use crate::http::{Response, Status};

impl From<Status> for axum::http::StatusCode {
    fn from(status: Status) -> Self {
        axum::http::StatusCode::from_u16(status.as_u16()).expect("invalid HTTP status code")
    }
}

impl From<Response<axum::http::HeaderMap, axum::body::Body>>
    for axum::http::Response<axum::body::Body>
{
    fn from(response: Response<axum::http::HeaderMap, axum::body::Body>) -> Self {
        let (status, headers, body) = response.into_parts();

        let mut response = axum::http::Response::new(axum::body::Body::from(()));

        if let Some(b) = body {
            let mut response = axum::http::Response::new(b);
            *response.status_mut() = status.into();
            *response.headers_mut() = headers;

            return response;
        }

        *response.status_mut() = status.into();
        *response.headers_mut() = headers;

        response
    }
}
