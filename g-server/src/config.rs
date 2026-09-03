/// Server's config
#[derive(Clone)]
pub struct Config {
    /// Timeout in ms, default 5000 ms
    pub timeout: Option<u64>,
    /// Max concurrent requests
    pub concurrency_limit: Option<usize>,
    /// Request body limit in KiB, default: 10240 KiB
    pub body_limit: Option<usize>,
    /// Response body compression method: default all
    pub compression: Option<Compression>,
}

impl Config {
    pub fn empty() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: Some(5000),
            concurrency_limit: Some(100_000),
            body_limit: Some(10240),
            compression: Some(Compression::All),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Compression {
    Deflate,
    Gzip,
    Brotli,
    Zstd,
    #[default]
    All, // prioritization: zstd, brotli, gzip, deflate.
}
