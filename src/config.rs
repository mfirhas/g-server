/// Server's config
#[derive(Clone)]
pub struct Config<C = ()> {
    /// Timeout in ms, default 5000 ms
    pub timeout: u32,
    /// Max concurrent requests
    pub concurrency_limit: u32,
    /// Request body limit in MiB, default: 10 MiB
    pub body_limit: u64,
    /// Response body compression method: default all
    pub compression: Compression,
    /// Max time an idle connections stay open, in ms, default: 1000 ms
    pub keep_alive: u32,
    /// Application context, shared for all handlers and middlewares under its group
    pub app_context: C,
}

impl<C> Config<C> {
    pub fn with_context<AC>(self, app_context: AC) -> Config<AC>
    where
        AC: Clone + Send + Sync + 'static,
    {
        Config {
            timeout: self.timeout,
            concurrency_limit: self.concurrency_limit,
            body_limit: self.body_limit,
            compression: self.compression,
            keep_alive: self.keep_alive,
            app_context,
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
