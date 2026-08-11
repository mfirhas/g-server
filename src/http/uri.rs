use std::fmt;

/// An HTTP request URI.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Uri(String);

impl Uri {
    pub fn new(uri: impl Into<String>) -> Self {
        Self(uri.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for Uri {
    fn from(uri: String) -> Self {
        Self::new(uri)
    }
}

impl From<&str> for Uri {
    fn from(uri: &str) -> Self {
        Self::new(uri)
    }
}
