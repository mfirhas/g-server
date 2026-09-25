/// Server's config
#[derive(Debug, Default)]
pub struct Config {
    /// Timeout in ms, default 5000 ms
    pub timeout: Option<u64>,
    /// Max concurrent requests
    pub concurrency_limit: Option<usize>,
    /// Request body limit in bytes, default: 2 MiB
    pub body_limit: Option<usize>,
    /// Response body compression method: default all
    pub compression: Option<Compression>,
    /// Remove repeated slash(es)
    pub normalize_endpoint: Option<bool>,

    /// Custom timeout error
    pub timeout_error: Option<fn() -> ::axum::response::Response>,
    /// Custom concurrency limit error
    pub concurrency_limit_error: Option<fn() -> ::axum::response::Response>,
    /// Custom bad request error
    pub bad_request_error: Option<fn(&str) -> ::axum::response::Response>,
    /// Fallback error
    pub fallback_error: Option<fn() -> ::axum::response::Response>,

    /// Cors
    pub cors: Option<Cors>,

    // file server configs
    /// directory to be served
    pub dir: Option<&'static str>,
    /// fallback file in case file not found
    pub fallback_file: Option<&'static str>,
    /// toggle filesystem embed,
    /// `dir` then is embedded into binary
    pub embed: Option<bool>,
}

impl Config {
    pub fn empty() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cors {
    pub allowed_origins: Option<Vec<crate::http::HeaderValue>>,
    pub allowed_methods: Option<Vec<crate::http::Method>>,
    pub allowed_headers: Option<Vec<crate::http::HeaderName>>,
    pub exposed_headers: Option<Vec<crate::http::HeaderName>>,
    pub allow_credentials: Option<bool>,
    pub max_age: Option<u64>, // in seconds
}

impl Cors {
    pub fn layer(cors: Cors) -> crate::tower_http::cors::CorsLayer {
        use tower_http::cors::CorsLayer;

        let mut layer = CorsLayer::new();

        if let Some(origins) = cors.allowed_origins {
            layer = layer.allow_origin(origins);
        }

        if let Some(methods) = cors.allowed_methods {
            layer = layer.allow_methods(methods);
        }

        if let Some(headers) = cors.allowed_headers {
            layer = layer.allow_headers(headers);
        }

        if let Some(headers) = cors.exposed_headers {
            layer = layer.expose_headers(headers);
        }

        if let Some(credentials) = cors.allow_credentials {
            layer = layer.allow_credentials(credentials);
        }

        if let Some(max_age) = cors.max_age {
            layer = layer.max_age(std::time::Duration::from_secs(max_age));
        }

        layer
    }
}

/// Compression provided by http server.
///
/// No compression for response body below 32 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    Deflate,
    Gzip,
    Brotli,
    Zstd,
    #[default]
    All, // prioritization: zstd, brotli, gzip, deflate.
}
