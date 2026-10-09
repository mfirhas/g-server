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

    /// Logging config
    pub logging: Option<Logging>,

    /// Tracing config
    pub tracing: Option<Tracing>,

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

    /// Request id
    pub request_id: Option<RequestId>,

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
            request_id: self.request_id,
            cors: self.cors,
            dir: self.dir,
            fallback_file: self.fallback_file,
            embed: self.embed,
            graceful_shutdown: self.graceful_shutdown,
            logging: self.logging,
            tracing: self.tracing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestId {
    pub id: RequestIdType,
    pub header: crate::http::HeaderName,
}

const DEFAULT_REQUEST_ID_HEADER: &str = "x-request-id";

impl Default for RequestId {
    #[inline]
    fn default() -> Self {
        Self {
            id: RequestIdType::UUIDv4,
            header: crate::http::HeaderName::from_static(DEFAULT_REQUEST_ID_HEADER),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RequestIdType {
    UUIDv4,
    UUIDv7,
    Custom(fn() -> Result<crate::http::HeaderValue, String>),
}

impl RequestId {
    pub fn try_new_req_id(&self) -> Result<crate::http::HeaderValue, String> {
        match self.id {
            RequestIdType::UUIDv4 => {
                crate::http::HeaderValue::from_str(crate::uuid::Uuid::new_v4().to_string().as_str())
                    .map_err(|err| err.to_string())
            }
            RequestIdType::UUIDv7 => {
                crate::http::HeaderValue::from_str(crate::uuid::Uuid::now_v7().to_string().as_str())
                    .map_err(|err| err.to_string())
            }
            RequestIdType::Custom(val) => val(),
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
    /// list of client ca's certs for mTLS
    pub client_cas: Option<Vec<String>>,
}

impl Tls {
    #[cfg(feature = "tls")]
    pub async fn tls_config(&self) -> Result<crate::axum_server::tls_rustls::RustlsConfig, String> {
        crate::axum_server::tls_rustls::RustlsConfig::from_pem_file(&self.cert, &self.key)
            .await
            .map_err(|err| err.to_string())
    }

    #[cfg(feature = "tls")]
    pub fn mtls_config(&self) -> Result<crate::axum_server::tls_rustls::RustlsConfig, String> {
        use crate::rustls::{RootCertStore, ServerConfig, server::WebPkiClientVerifier};
        use crate::rustls_pemfile::{certs, private_key};

        if self.client_cas.is_none() {
            return Err("client cas are empty".into());
        }

        let client_cas = if let Some(cas) = &self.client_cas
            && !cas.is_empty()
        {
            cas
        } else {
            return Err("client cas are empty".into());
        };

        let mut roots = RootCertStore::empty();

        for ca_path in client_cas {
            let file = std::fs::File::open(ca_path).map_err(|err| err.to_string())?;
            let mut reader = std::io::BufReader::new(file);

            for cert in certs(&mut reader) {
                roots
                    .add(cert.map_err(|err| err.to_string())?)
                    .map_err(|err| err.to_string())?;
            }
        }

        let verifier = WebPkiClientVerifier::builder(std::sync::Arc::new(roots))
            .build()
            .map_err(|err| err.to_string())?;

        let server_cert = std::fs::File::open(&self.cert).map_err(|err| err.to_string())?;
        let mut server_cert = std::io::BufReader::new(server_cert);

        let server_key = std::fs::File::open(&self.key).map_err(|err| err.to_string())?;
        let mut server_key = std::io::BufReader::new(server_key);

        let mut config = ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                certs(&mut server_cert)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|err| err.to_string())?,
                private_key(&mut server_key)
                    .map_err(|err| err.to_string())?
                    .ok_or_else(|| std::io::Error::other("no private key found"))
                    .map_err(|err| err.to_string())?,
            )
            .map_err(|err| err.to_string())?;

        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(crate::axum_server::tls_rustls::RustlsConfig::from_config(
            std::sync::Arc::new(config),
        ))
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

// WARN: PUBLIC
/// Logging configs
#[derive(Debug, Clone)]
pub struct Logging {
    /// Max log level.
    ///
    /// All levels above this won't be logged.
    pub level: LogLevel,
    /// Logging format: default(normal oneline log), pretty(multiline), json.
    pub format: LogFormat,
    /// Timestamp offset: utc(0) or local
    pub time_offset: LogTimeOffset,

    /// `init()` function to initializes the logger
    pub init_fn: fn(LoggerInitParams) -> Result<(), String>,
}

impl Logging {
    pub fn init(self) -> Result<(), String> {
        println!("g-server: initializing logging...");
        (self.init_fn)(self.into())
    }
}

#[derive(Debug, Clone)]
pub struct LoggerInitParams {
    /// Max log level.
    ///
    /// All levels above this won't be logged.
    pub level: LogLevel,
    /// Logging format: default(normal oneline log), pretty(multiline), json.
    pub format: LogFormat,
    /// Timestamp offset: utc(0) or local
    pub time_offset: LogTimeOffset,
}

impl From<Logging> for LoggerInitParams {
    #[inline]
    fn from(value: Logging) -> Self {
        LoggerInitParams {
            level: value.level,
            format: value.format,
            time_offset: value.time_offset,
        }
    }
}

pub fn default_logger_init(params: LoggerInitParams) -> Result<(), String> {
    use std::io::Write;

    let mut builder = env_logger::Builder::new();

    builder
        .filter_level(params.level.into())
        .format(move |buf, record| {
            let timestamp = match params.time_offset {
                LogTimeOffset::UTC => chrono::Utc::now().to_rfc3339(),
                LogTimeOffset::Local => chrono::Local::now().to_rfc3339(),
            };

            match params.format {
                LogFormat::Default => {
                    writeln!(buf, "{} [{}] {}", timestamp, record.level(), record.args(),)
                }

                LogFormat::Pretty => writeln!(
                    buf,
                    "\n{}\n  level: {}\n  target: {}\n  message: {}\n",
                    timestamp,
                    record.level(),
                    record.target(),
                    record.args(),
                ),

                LogFormat::Json => {
                    let message = record.args().to_string();

                    serde_json::to_writer(
                        &mut *buf,
                        &serde_json::json!({
                            "timestamp": timestamp,
                            "level": record.level().to_string(),
                            "target": record.target(),
                            "message": message,
                        }),
                    )
                    .map_err(std::io::Error::other)?;

                    writeln!(buf)
                }
            }
        });

    builder.try_init().map_err(|err| err.to_string())
}

impl Default for Logging {
    fn default() -> Self {
        Logging {
            level: LogLevel::default(),
            format: LogFormat::default(),
            time_offset: LogTimeOffset::default(),
            init_fn: default_logger_init,
        }
    }
}

/// Tracing configs
#[derive(Debug, Clone)]
pub struct Tracing {
    /// Setup maximum tracing level: error -> warn -> info -> debug, -> trace,
    ///
    /// error is minimum level.
    ///
    /// trace is maximum level.
    ///
    /// RUST_LOG is listened, then smallest wins.
    ///
    /// Default is `info`
    pub level: LogLevel,

    /// Tracing logs format:
    ///
    /// - `default`: normal one-line log format. (default)
    /// - `pretty`: pretty multi-line log format.
    /// - `json`: json format
    pub format: LogFormat,

    /// Trace logs timestamp offset.
    ///
    /// Time format is RFC 3339.
    ///
    /// Defaults to `utc`.
    pub time_offset: LogTimeOffset,

    pub trace_log: bool,
}

impl Default for Tracing {
    fn default() -> Self {
        Tracing {
            level: LogLevel::default(),
            format: LogFormat::default(),
            time_offset: LogTimeOffset::default(),
            trace_log: true,
        }
    }
}

impl Tracing {
    #[cfg(feature = "tracing")]
    pub fn init(&self) -> Result<(), String> {
        use crate::tracing;
        use crate::tracing_subscriber::{self, layer::SubscriberExt};

        println!("g-server: initializing tracing...");

        let level = match self.level {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        };

        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level.as_str()));

        let span_events = tracing_subscriber::fmt::format::FmtSpan::CLOSE;

        let utc_timer = tracing_subscriber::fmt::time::ChronoUtc::rfc_3339();

        let local_timer = tracing_subscriber::fmt::time::ChronoLocal::rfc_3339();

        let fmt_layer_utc_default = tracing_subscriber::fmt::layer()
            .with_span_events(span_events.clone())
            .with_timer(utc_timer.clone());
        let fmt_layer_utc_pretty = tracing_subscriber::fmt::layer()
            .with_span_events(span_events.clone())
            .with_timer(utc_timer.clone())
            .pretty();
        let fmt_layer_utc_json = tracing_subscriber::fmt::layer()
            .with_span_events(span_events.clone())
            .with_timer(utc_timer)
            .json();

        let fmt_layer_local_default = tracing_subscriber::fmt::layer()
            .with_span_events(span_events.clone())
            .with_timer(local_timer.clone());
        let fmt_layer_local_pretty = tracing_subscriber::fmt::layer()
            .with_span_events(span_events.clone())
            .with_timer(local_timer.clone())
            .pretty();
        let fmt_layer_local_json = tracing_subscriber::fmt::layer()
            .with_span_events(span_events)
            .with_timer(local_timer)
            .json();

        let _ = match (self.time_offset, self.format) {
            (LogTimeOffset::UTC, LogFormat::Default) => {
                let subscriber = tracing_subscriber::Registry::default()
                    .with(env_filter)
                    .with(fmt_layer_utc_default);

                tracing::subscriber::set_global_default(subscriber)
                    .map_err(|err| err.to_string())?
            }
            (LogTimeOffset::UTC, LogFormat::Pretty) => {
                let subscriber = tracing_subscriber::Registry::default()
                    .with(env_filter)
                    .with(fmt_layer_utc_pretty);

                tracing::subscriber::set_global_default(subscriber)
                    .map_err(|err| err.to_string())?
            }
            (LogTimeOffset::UTC, LogFormat::Json) => {
                let subscriber = tracing_subscriber::Registry::default()
                    .with(env_filter)
                    .with(fmt_layer_utc_json);

                tracing::subscriber::set_global_default(subscriber)
                    .map_err(|err| err.to_string())?
            }

            (LogTimeOffset::Local, LogFormat::Default) => {
                let subscriber = tracing_subscriber::Registry::default()
                    .with(env_filter)
                    .with(fmt_layer_local_default);

                tracing::subscriber::set_global_default(subscriber)
                    .map_err(|err| err.to_string())?
            }
            (LogTimeOffset::Local, LogFormat::Pretty) => {
                let subscriber = tracing_subscriber::Registry::default()
                    .with(env_filter)
                    .with(fmt_layer_local_pretty);

                tracing::subscriber::set_global_default(subscriber)
                    .map_err(|err| err.to_string())?
            }
            (LogTimeOffset::Local, LogFormat::Json) => {
                let subscriber = tracing_subscriber::Registry::default()
                    .with(env_filter)
                    .with(fmt_layer_local_json);

                tracing::subscriber::set_global_default(subscriber)
                    .map_err(|err| err.to_string())?
            }
        };

        if self.trace_log {
            crate::tracing_log::LogTracer::init().map_err(|err| err.to_string())?
        }

        Ok(())
    }
}

/// Tracing levels
///
/// The order from top to bottom is from less verbose to most verbose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for log::LevelFilter {
    #[inline]
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Self::Error,
            LogLevel::Warn => Self::Warn,
            LogLevel::Info => Self::Info,
            LogLevel::Debug => Self::Debug,
            LogLevel::Trace => Self::Trace,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    #[default]
    Default,
    Pretty,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogTimeOffset {
    #[default]
    UTC,
    Local,
}
