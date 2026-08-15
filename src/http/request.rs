use super::{body::Body, header::Header, method::Method, uri::Uri};

/// An HTTP request.
#[derive(Debug)]
pub struct Request<URI, H, B> {
    method: Method,
    uri: URI,
    header: H,
    body: Option<B>,
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
        let m = method.into();
        let b: Option<B> = match m {
            // bodyless verbs
            Method::Get
            | Method::Head
            | Method::Delete
            | Method::Options
            | Method::Trace
            | Method::Connect => None,
            _ => Some(body),
        };

        Self {
            method: m,
            uri,
            header,
            body: b,
        }
    }

    pub fn set_body(&mut self, body: B) {
        self.body = Some(body)
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

    pub fn body(&self) -> Option<&B> {
        self.body.as_ref()
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

    pub fn body_mut(&mut self) -> Option<&mut B> {
        self.body.as_mut()
    }

    pub fn into_body(self) -> Option<B> {
        self.body
    }

    pub fn into_parts(self) -> (Method, URI, H, Option<B>) {
        (self.method, self.uri, self.header, self.body)
    }
}
