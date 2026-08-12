use std::borrow::Cow;

use crate::http::Method;

pub struct Route<H> {
    method: Method,
    path: Cow<'static, str>,
    handler: H,
}

impl<H> Route<H> {
    pub fn new(method: Method, path: impl Into<Cow<'static, str>>, handler: H) -> Self {
        Self {
            method,
            path: path.into(),
            handler,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn handler(&self) -> &H {
        &self.handler
    }

    pub fn into_handler(self) -> H {
        self.handler
    }
}
