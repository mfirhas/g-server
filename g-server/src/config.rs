use std::fmt::Debug;
use std::hash::Hash;

/// Server's config
#[derive(Debug, Default)]
pub struct Config<RLKey = ()> {
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
    /// toggle graceful shutdown
    pub graceful_shutdown: Option<bool>,

    /// TLS config
    pub tls: Option<Tls>,

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

    /// rate limit using GCRA(leaky bucket)
    pub rate_limit: Option<RateLimit<RLKey>>,

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

    pub fn with_rate_limit<RLKey>(self, rate_limit_key: RateLimit<RLKey>) -> Config<RLKey> {
        Config::<RLKey> {
            rate_limit: Some(rate_limit_key),

            tls: self.tls,

            timeout: self.timeout,
            concurrency_limit: self.concurrency_limit,
            body_limit: self.body_limit,
            compression: self.compression,
            normalize_endpoint: self.normalize_endpoint,
            timeout_error: self.timeout_error,
            concurrency_limit_error: self.concurrency_limit_error,
            bad_request_error: self.bad_request_error,
            fallback_error: self.fallback_error,
            cors: self.cors,
            dir: self.dir,
            fallback_file: self.fallback_file,
            embed: self.embed,
            graceful_shutdown: self.graceful_shutdown,
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
    pub fn layer(self) -> crate::tower_http::cors::CorsLayer {
        use tower_http::cors::CorsLayer;

        let mut layer = CorsLayer::new();

        if let Some(origins) = self.allowed_origins {
            layer = layer.allow_origin(origins);
        }

        if let Some(methods) = self.allowed_methods {
            layer = layer.allow_methods(methods);
        }

        if let Some(headers) = self.allowed_headers {
            layer = layer.allow_headers(headers);
        }

        if let Some(headers) = self.exposed_headers {
            layer = layer.expose_headers(headers);
        }

        if let Some(credentials) = self.allow_credentials {
            layer = layer.allow_credentials(credentials);
        }

        if let Some(max_age) = self.max_age {
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

#[derive(Debug, Clone)]
pub struct RateLimit<Key = ()> {
    /// burst size allowed,
    /// must not be zero
    pub burst_size: u32,
    /// interval in which a token refilled in milliseconds,
    /// must not be zero,
    /// defaults to 100ms
    pub interval: u64,
    /// return rate limit headers,
    /// defaults to false
    pub with_headers: bool,
    /// rate limit key,
    /// default to IP,
    /// IP forwarding is supported
    pub key: RateLimitKey<Key>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Default)]
pub enum RateLimitKey<Key = ()> {
    Global,
    #[default]
    IP,
    Custom(Key),
}

impl<Key> RateLimit<Key> {
    #[cfg(feature = "ratelimit")]
    pub fn layer<C>(&self, router: crate::axum::Router<C>) -> crate::axum::Router<C>
    where
        Key: Debug + Eq + PartialEq + Clone + Hash + CustomKey + Send + Sync + 'static,
        C: Clone + Send + Sync + 'static,
    {
        match (self.with_headers, &self.key) {
            (true, RateLimitKey::Global) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .use_headers()
                    .key_extractor(crate::tower_governor::key_extractor::GlobalKeyExtractor)
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (true, RateLimitKey::IP) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .use_headers()
                    .key_extractor(crate::tower_governor::key_extractor::SmartIpKeyExtractor)
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (false, RateLimitKey::Global) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .key_extractor(crate::tower_governor::key_extractor::GlobalKeyExtractor)
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (false, RateLimitKey::IP) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .key_extractor(crate::tower_governor::key_extractor::SmartIpKeyExtractor)
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (true, RateLimitKey::Custom(key)) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .use_headers()
                    .key_extractor(CustomRateLimitKey(key.clone()))
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (false, RateLimitKey::Custom(key)) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .key_extractor(CustomRateLimitKey(key.clone()))
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
        }
    }

    #[cfg(feature = "ratelimit")]
    pub fn route_layer<C>(
        &self,
        router: crate::axum::routing::MethodRouter<C>,
    ) -> crate::axum::routing::MethodRouter<C>
    where
        Key: Debug + Eq + PartialEq + Clone + Hash + CustomKey + Send + Sync + 'static,
        C: Clone + Send + Sync + 'static,
    {
        match (self.with_headers, &self.key) {
            (true, RateLimitKey::Global) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .use_headers()
                    .key_extractor(crate::tower_governor::key_extractor::GlobalKeyExtractor)
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.route_layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (true, RateLimitKey::IP) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .use_headers()
                    .key_extractor(crate::tower_governor::key_extractor::SmartIpKeyExtractor)
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.route_layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (false, RateLimitKey::Global) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .key_extractor(crate::tower_governor::key_extractor::GlobalKeyExtractor)
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.route_layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (false, RateLimitKey::IP) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .key_extractor(crate::tower_governor::key_extractor::SmartIpKeyExtractor)
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.route_layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (true, RateLimitKey::Custom(key)) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .use_headers()
                    .key_extractor(CustomRateLimitKey(key.clone()))
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.route_layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
            (false, RateLimitKey::Custom(key)) => {
                let conf = crate::tower_governor::governor::GovernorConfigBuilder::default()
                    .burst_size(self.burst_size)
                    .per_millisecond(self.interval)
                    .key_extractor(CustomRateLimitKey(key.clone()))
                    .finish()
                    .expect("g-server: RateLimit::layer: invalid rate limit params");

                router.route_layer(crate::tower_governor::GovernorLayer::new(
                    std::sync::Arc::new(conf),
                ))
            }
        }
    }
}

#[derive(Clone)]
pub struct CustomRateLimitKey<Key>(Key);

pub trait CustomKey {
    type Key: Debug + Eq + PartialEq + Clone + Hash + Send + Sync + 'static;

    fn key<B>(&self, req: &crate::http::Request<B>) -> Result<Self::Key, CustomKeyError>;
}

impl CustomKey for () {
    type Key = ();

    fn key<B>(&self, _req: &crate::http::Request<B>) -> Result<Self::Key, CustomKeyError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CustomKeyError {
    pub status: crate::StatusCode,
    pub message: String,
}

#[cfg(feature = "ratelimit")]
impl<CKey> crate::tower_governor::key_extractor::KeyExtractor for CustomRateLimitKey<CKey>
where
    CKey: Debug + Eq + PartialEq + Clone + Hash + CustomKey + Send + Sync + 'static,
{
    type Key = CKey::Key;

    fn extract<B>(
        &self,
        req: &http::Request<B>,
    ) -> Result<Self::Key, tower_governor::GovernorError> {
        self.0
            .key(req)
            .map_err(|err| crate::tower_governor::GovernorError::Other {
                code: err.status,
                msg: Some(err.message),
                headers: None,
            })
    }
}

// TLS configs
#[derive(Debug, Clone)]
pub struct Tls {
    /// file path to certificate
    pub cert: String,
    /// file path to private key
    pub key: String,
    /// http port as source of redirection to https
    pub redirect_from_port: Option<u16>,
}

impl Tls {
    #[cfg(feature = "tls")]
    pub async fn rustls_config(
        &self,
    ) -> Result<crate::axum_server::tls_rustls::RustlsConfig, String> {
        crate::axum_server::tls_rustls::RustlsConfig::from_pem_file(&self.cert, &self.key)
            .await
            .map_err(|err| err.to_string())
    }

    #[cfg(feature = "tls")]
    pub fn redirect_http_to_https(
        host: &'static str,
        http_port: u16,
        https_port: u16,
    ) -> impl Future<Output = std::io::Result<()>> {
        fn make_https(
            uri: crate::axum::http::Uri,
            host: &'static str,
            https_port: u16,
        ) -> Result<crate::axum::http::Uri, crate::axum::BoxError> {
            let mut parts = uri.into_parts();

            parts.scheme = Some(crate::axum::http::uri::Scheme::HTTPS);
            parts.authority = Some(format!("{host}:{https_port}").parse()?);

            if parts.path_and_query.is_none() {
                parts.path_and_query = Some("/".parse().unwrap());
            }

            Ok(crate::axum::http::Uri::from_parts(parts)?)
        }

        async move {
            let redirect = move |uri: crate::axum::http::Uri| async move {
                match make_https(uri, host, https_port) {
                    Ok(uri) => Ok(crate::axum::response::Redirect::permanent(&uri.to_string())),
                    Err(err) => Err((crate::axum::http::StatusCode::BAD_REQUEST, err.to_string())),
                }
            };

            let listener = crate::tokio::net::TcpListener::bind((host, http_port)).await?;

            crate::axum::serve(
                listener,
                crate::axum::routing::any(redirect).into_make_service(),
            )
            .await
        }
    }
}
