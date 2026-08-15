use crate::http::{Body, Header, Method, Request, Uri};
use axum::body::Body as AxumBody;
use http_body_util::BodyExt;
use serde::{Serialize, de::DeserializeOwned};

impl From<axum::http::Method> for Method {
    fn from(method: axum::http::Method) -> Self {
        match method {
            axum::http::Method::GET => Self::Get,
            axum::http::Method::POST => Self::Post,
            axum::http::Method::PUT => Self::Put,
            axum::http::Method::PATCH => Self::Patch,
            axum::http::Method::DELETE => Self::Delete,
            axum::http::Method::HEAD => Self::Head,
            axum::http::Method::OPTIONS => Self::Options,
            axum::http::Method::CONNECT => Self::Connect,
            axum::http::Method::TRACE => Self::Trace,
            axum::http::Method::QUERY => Self::Query,
            _ => Self::Any,
        }
    }
}

impl Uri for axum::http::Uri {
    fn scheme(&self) -> Option<&str> {
        self.scheme_str()
    }

    fn authority(&self) -> Option<&str> {
        self.authority().map(|authority| authority.as_str())
    }

    fn path(&self) -> &str {
        self.path()
    }

    fn query(&self) -> Option<&str> {
        self.query()
    }

    fn fragment(&self) -> Option<&str> {
        None
    }
}

impl Header for axum::http::HeaderMap {
    fn get(&self, name: &str) -> Option<&[u8]> {
        axum::http::HeaderMap::get(self, name).map(|value| value.as_bytes())
    }

    fn get_all(&self, name: &str) -> impl Iterator<Item = &[u8]> {
        axum::http::HeaderMap::get_all(self, name)
            .iter()
            .map(|value| value.as_bytes())
    }

    fn contains(&self, name: &str) -> bool {
        axum::http::HeaderMap::contains_key(self, name)
    }

    fn is_empty(&self) -> bool {
        axum::http::HeaderMap::is_empty(self)
    }

    fn len(&self) -> usize {
        axum::http::HeaderMap::len(self)
    }
}

impl Body for AxumBody {
    type Error = axum::Error;

    async fn to_text(self) -> Result<String, Self::Error> {
        let bytes: Vec<u8> = self.collect().await?.to_bytes().into();

        String::from_utf8(bytes).map_err(axum::Error::new)
    }

    async fn to_json<T>(self) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        let bytes = self.collect().await?.to_bytes();

        ::serde_json::from_slice(&bytes).map_err(axum::Error::new)
    }

    fn from_text(text: String) -> Result<Self, Self::Error> {
        Ok(Self::from(text))
    }

    fn from_json<T>(value: T) -> Result<Self, Self::Error>
    where
        T: Serialize,
    {
        let bytes = ::serde_json::to_vec(&value).map_err(axum::Error::new)?;

        Ok(Self::from(bytes))
    }
}

impl From<axum::http::Request<axum::body::Body>>
    for Request<axum::http::Uri, axum::http::HeaderMap, AxumBody>
{
    fn from(request: axum::http::Request<axum::body::Body>) -> Self {
        let (parts, body) = request.into_parts();

        Self::new(parts.method, parts.uri, parts.headers, body)
    }
}
