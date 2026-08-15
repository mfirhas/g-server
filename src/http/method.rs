use std::fmt;

/// An HTTP request method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Connect,
    Trace,
    Query,
    Any,
}

impl Method {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Connect => "CONNECT",
            Self::Trace => "TRACE",
            Self::Query => "QUERY",
            Self::Any => "*",
        }
    }

    pub fn is_get(&self) -> bool {
        matches!(self, Self::Get)
    }

    pub fn is_post(&self) -> bool {
        matches!(self, Self::Post)
    }

    pub fn is_put(&self) -> bool {
        matches!(self, Self::Put)
    }

    pub fn is_patch(&self) -> bool {
        matches!(self, Self::Patch)
    }

    pub fn is_delete(&self) -> bool {
        matches!(self, Self::Delete)
    }

    pub fn is_head(&self) -> bool {
        matches!(self, Self::Head)
    }

    pub fn is_options(&self) -> bool {
        matches!(self, Self::Options)
    }

    pub fn is_connect(&self) -> bool {
        matches!(self, Self::Connect)
    }

    pub fn is_trace(&self) -> bool {
        matches!(self, Self::Trace)
    }

    pub fn is_query(&self) -> bool {
        matches!(self, Self::Query)
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
