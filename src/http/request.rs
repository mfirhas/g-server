use super::{body::Body, header::Header, method::Method, uri::Uri};

/// An HTTP request.
#[derive(Debug)]
pub struct Request {
    method: Method,
    uri: Uri,
    headers: Header,
    body: Body,
}

impl Request {
    pub fn new(
        method: impl Into<Method>,
        uri: impl Into<Uri>,
        headers: Header,
        body: impl Into<Body>,
    ) -> Self {
        Self {
            method: method.into(),
            uri: uri.into(),
            headers,
            body: body.into(),
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    pub fn headers(&self) -> &Header {
        &self.headers
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.method
    }

    pub fn uri_mut(&mut self) -> &mut Uri {
        &mut self.uri
    }

    pub fn headers_mut(&mut self) -> &mut Header {
        &mut self.headers
    }

    pub fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }

    pub fn into_body(self) -> Body {
        self.body
    }

    pub fn into_parts(self) -> (Method, Uri, Header, Body) {
        (self.method, self.uri, self.headers, self.body)
    }
}
