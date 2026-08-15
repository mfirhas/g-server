use super::{Method, body::Body, header::Header, status::Status};

/// An HTTP response.
#[derive(Debug)]
pub struct Response<H, B> {
    status: Status,
    headers: H,
    body: Option<B>,
}

impl<H, B> Response<H, B>
where
    H: Header,
    B: Body,
{
    /// Creates a new HTTP response.
    pub fn new<S>(method: Method, status: S, headers: H, body: B) -> Self
    where
        S: Into<Status>,
    {
        let s = status.into();
        let b: Option<B> = match (method, s, s.is_informational()) {
            (Method::Head, _, _) => None,
            (_, Status::NO_CONTENT | Status::NOT_MODIFIED, _) => None,
            (_, _, true) => None,
            _ => Some(body),
        };

        Self {
            status: s,
            headers,
            body: b,
        }
    }

    /// Returns the response status.
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Returns the response headers.
    pub fn headers(&self) -> &H {
        &self.headers
    }

    /// Returns the response body.
    pub fn body(&self) -> Option<&B> {
        self.body.as_ref()
    }

    pub fn set_body(&mut self, body: B) {
        self.body = Some(body)
    }

    /// Returns a mutable reference to the response status.
    pub fn status_mut(&mut self) -> &mut Status {
        &mut self.status
    }

    /// Returns a mutable reference to the response headers.
    pub fn headers_mut(&mut self) -> &mut H {
        &mut self.headers
    }

    /// Returns a mutable reference to the response body.
    pub fn body_mut(&mut self) -> Option<&mut B> {
        self.body.as_mut()
    }

    /// Consumes the response and returns its body.
    pub fn into_body(self) -> Option<B> {
        self.body
    }

    /// Consumes the response and returns its components.
    pub fn into_parts(self) -> (Status, H, Option<B>) {
        (self.status, self.headers, self.body)
    }
}

// into response

pub trait IntoResponse {
    type Response;

    fn into_response(self) -> Self::Response;
}

impl<H, B> IntoResponse for Response<H, B>
where
    H: Header,
    B: Body,
{
    type Response = Response<H, B>;

    fn into_response(self) -> Self::Response {
        self
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: IntoResponse<Response = T::Response>,
{
    type Response = T::Response;

    fn into_response(self) -> Self::Response {
        match self {
            Ok(value) => value.into_response(),
            Err(error) => error.into_response(),
        }
    }
}
