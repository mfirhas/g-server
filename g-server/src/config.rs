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
}

impl Config {
    pub fn empty() -> Self {
        Self {
            ..Default::default()
        }
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
