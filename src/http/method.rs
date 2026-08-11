use std::{borrow::Cow, fmt};

/// An HTTP request method.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Method(Cow<'static, str>);

impl Method {
    pub const GET: Self = Self(Cow::Borrowed("GET"));
    pub const POST: Self = Self(Cow::Borrowed("POST"));
    pub const PUT: Self = Self(Cow::Borrowed("PUT"));
    pub const PATCH: Self = Self(Cow::Borrowed("PATCH"));
    pub const DELETE: Self = Self(Cow::Borrowed("DELETE"));
    pub const HEAD: Self = Self(Cow::Borrowed("HEAD"));
    pub const OPTIONS: Self = Self(Cow::Borrowed("OPTIONS"));
    pub const CONNECT: Self = Self(Cow::Borrowed("CONNECT"));
    pub const TRACE: Self = Self(Cow::Borrowed("TRACE"));

    /// Creates a custom HTTP method.
    pub fn new(method: impl Into<String>) -> Self {
        Self(Cow::Owned(method.into()))
    }

    /// Returns the method as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_get(&self) -> bool {
        self == &Self::GET
    }

    pub fn is_post(&self) -> bool {
        self == &Self::POST
    }

    pub fn is_put(&self) -> bool {
        self == &Self::PUT
    }

    pub fn is_patch(&self) -> bool {
        self == &Self::PATCH
    }

    pub fn is_delete(&self) -> bool {
        self == &Self::DELETE
    }

    pub fn is_head(&self) -> bool {
        self == &Self::HEAD
    }

    pub fn is_options(&self) -> bool {
        self == &Self::OPTIONS
    }

    pub fn is_connect(&self) -> bool {
        self == &Self::CONNECT
    }

    pub fn is_trace(&self) -> bool {
        self == &Self::TRACE
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for Method {
    fn from(method: String) -> Self {
        Self::new(method)
    }
}

impl From<&str> for Method {
    fn from(method: &str) -> Self {
        Self::new(method)
    }
}
