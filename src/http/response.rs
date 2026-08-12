use super::{body::Body, header::Header, status::Status};

/// An HTTP response.
#[derive(Debug)]
pub struct Response<H, B> {
    status: Status,
    headers: H,
    body: B,
}

impl<H, B> Response<H, B>
where
    H: Header,
    B: Body,
{
    /// Creates a new HTTP response.
    pub fn new<S>(status: S, headers: H, body: B) -> Self
    where
        S: Into<Status>,
    {
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
    pub fn headers(&self) -> &H {
        &self.headers
    }

    /// Returns the response body.
    pub fn body(&self) -> &B {
        &self.body
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
    pub fn body_mut(&mut self) -> &mut B {
        &mut self.body
    }

    /// Consumes the response and returns its body.
    pub fn into_body(self) -> B {
        self.body
    }

    /// Consumes the response and returns its components.
    pub fn into_parts(self) -> (Status, H, B) {
        (self.status, self.headers, self.body)
    }
}
