use super::{body::Body, header::Header, method::Method, uri::Uri};

/// An HTTP request.
#[derive(Debug)]
pub struct Request<URI, H, B> {
    method: Method,
    uri: URI,
    header: H,
    body: B,
}

impl<URI, H, B> Request<URI, H, B>
where
    URI: Uri,
    H: Header,
    B: Body,
{
    pub fn new<M>(method: M, uri: URI, header: H, body: B) -> Self
    where
        M: Into<Method>,
    {
        Self {
            method: method.into(),
            uri,
            header,
            body,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn uri(&self) -> &URI {
        &self.uri
    }

    pub fn headers(&self) -> &H {
        &self.header
    }

    pub fn body(&self) -> &B {
        &self.body
    }

    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.method
    }

    pub fn uri_mut(&mut self) -> &mut URI {
        &mut self.uri
    }

    pub fn headers_mut(&mut self) -> &mut H {
        &mut self.header
    }

    pub fn body_mut(&mut self) -> &mut B {
        &mut self.body
    }

    pub fn into_body(self) -> B {
        self.body
    }

    pub fn into_parts(self) -> (Method, URI, H, B) {
        (self.method, self.uri, self.header, self.body)
    }
}
