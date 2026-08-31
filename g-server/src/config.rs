/// Server's config
#[derive(Clone)]
pub struct Config {
    /// Timeout in ms, default 5000 ms
    pub timeout: Option<u64>,
    /// Max concurrent requests
    pub concurrency_limit: Option<u32>,
    /// Request body limit in KiB, default: 10240 KiB
    pub body_limit: Option<u64>,
    /// Response body compression method: default all
    pub compression: Option<Compression>,
    /// Max time an idle connections stay open, in ms, default: 1000 ms
    pub keep_alive: Option<u32>,
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
            keep_alive: Some(1000),
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
