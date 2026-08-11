use super::{body::Body, header::Header, status::Status};

/// An HTTP response.
#[derive(Debug)]
pub struct Response {
    status: Status,
    headers: Header,
    body: Body,
}

impl Response {
    /// Creates a new HTTP response.
    pub fn new(status: impl Into<Status>, headers: Header, body: impl Into<Body>) -> Self {
        Self {
            status: status.into(),
            headers,
            body: body.into(),
        }
    }

    /// Returns the response status.
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Returns the response headers.
    pub fn headers(&self) -> &Header {
        &self.headers
    }

    /// Returns the response body.
    pub fn body(&self) -> &Body {
        &self.body
    }

    /// Returns a mutable reference to the response status.
    pub fn status_mut(&mut self) -> &mut Status {
        &mut self.status
    }

    /// Returns a mutable reference to the response headers.
    pub fn headers_mut(&mut self) -> &mut Header {
        &mut self.headers
    }

    /// Returns a mutable reference to the response body.
    pub fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }

    /// Consumes the response and returns its body.
    pub fn into_body(self) -> Body {
        self.body
    }

    /// Consumes the response and returns its components.
    pub fn into_parts(self) -> (Status, Header, Body) {
        (self.status, self.headers, self.body)
    }
}
