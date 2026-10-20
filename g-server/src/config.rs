use tokio::fs;
use tokio::time::Instant;

use crate::axum::response::IntoResponse;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;

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

    /// Metrics config
    pub metrics: Option<Metrics>,

    /// TLS config
    pub tls: Option<Tls>,

    /// Panic error
    pub panic_error: Option<fn(&str) -> ::axum::response::Response>,
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
            panic_error: self.panic_error,
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
            metrics: self.metrics,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestId {
    pub id: RequestIdType,
    pub header: crate::http::HeaderName,
}

pub const CORRELATION_ID_HEADER: crate::http::HeaderName =
    crate::http::HeaderName::from_static("x-correlation-id");
pub const DEFAULT_REQUEST_ID_HEADER: crate::http::HeaderName =
    crate::http::HeaderName::from_static("x-request-id");

impl Default for RequestId {
    #[inline]
    fn default() -> Self {
        Self {
            id: RequestIdType::UUIDv4,
            header: DEFAULT_REQUEST_ID_HEADER,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum RequestIdType {
    #[default]
    UUIDv4,
    UUIDv7,
    Custom(fn() -> Result<crate::http::HeaderValue, String>),
}

impl RequestId {
    pub fn try_new_id(&self) -> Result<crate::http::HeaderValue, String> {
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

    pub async fn request_id_header_middleware(
        crate::axum::extract::State(req_id): crate::axum::extract::State<crate::config::RequestId>,
        mut request: crate::axum::extract::Request,
        next: crate::axum::middleware::Next,
    ) -> crate::axum::response::Response {
        let request_id = match request.headers_mut().entry(&req_id.header) {
            crate::http::header::Entry::Occupied(entry) => entry.get().clone(),
            crate::http::header::Entry::Vacant(entry) => {
                let value = match req_id.try_new_id() {
                    Ok(id) => id,
                    Err(err) => {
                        return (
                            crate::StatusCode::INTERNAL_SERVER_ERROR,
                            format!("g-server: failed creating new request id: {}", err),
                        )
                            .into_response();
                    }
                };
                entry.insert(value.clone());
                value
            }
        };

        let mut response = next.run(request).await;

        if let crate::http::header::Entry::Vacant(entry) =
            response.headers_mut().entry(&req_id.header)
        {
            entry.insert(request_id);
        }

        response
    }
}

#[derive(Debug, Clone)]
pub struct CorrelationId(RequestId);

impl CorrelationId {
    pub fn new_uuid_v7() -> Self {
        Self(RequestId {
            id: RequestIdType::UUIDv7,
            header: CORRELATION_ID_HEADER,
        })
    }

    pub fn new_custom(custom_fn: fn() -> Result<crate::http::HeaderValue, String>) -> Self {
        Self(RequestId {
            id: RequestIdType::Custom(custom_fn),
            header: CORRELATION_ID_HEADER,
        })
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self(RequestId {
            id: RequestIdType::UUIDv4,
            header: CORRELATION_ID_HEADER,
        })
    }
}

impl Deref for CorrelationId {
    type Target = RequestId;

    fn deref(&self) -> &Self::Target {
        &self.0
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
    /// Set logs target: stdout, stderr, or directly to file
    pub target: LogOutput,

    /// `init()` function to initializes the logger
    pub init_fn: fn(
        LoggerInitParams,
    ) -> std::pin::Pin<
        std::boxed::Box<dyn std::future::Future<Output = Result<(), String>>>,
    >,
}

impl Logging {
    pub async fn init(self, trace_config: &Option<Tracing>) -> Result<(), String> {
        println!("g-server: initializing logging...");

        if trace_config.is_none() {
            crate::tracing::subscriber::set_global_default(
                crate::tracing::subscriber::NoSubscriber::default(),
            )
            .map_err(|e| e.to_string())?;
        }

        (self.init_fn)(self.into()).await
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
    /// Set logs target: stdout, stderr, or directly to file
    pub target: LogOutput,
}

impl From<Logging> for LoggerInitParams {
    #[inline]
    fn from(value: Logging) -> Self {
        LoggerInitParams {
            level: value.level,
            format: value.format,
            time_offset: value.time_offset,
            target: value.target,
        }
    }
}

pub async fn default_logger_init(params: LoggerInitParams) -> Result<(), String> {
    use crate::env_logger::Target;
    use std::io::Write;

    let target = match &params.target {
        LogOutput::StdOut => Target::Stdout,
        LogOutput::StdErr => Target::Stderr,
        LogOutput::File(path) => {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|err| format!("failed to open log file {}: {err}", path.display()))?;
            Target::Pipe(Box::new(file))
        }
    };

    let mut builder = crate::env_logger::Builder::new();

    builder
        .filter_level(params.level.into())
        .target(target)
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
            target: LogOutput::default(),
            init_fn: |params| std::boxed::Box::pin(default_logger_init(params)),
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

    pub target: LogOutput,

    pub trace_log: bool,

    pub correlation_id: CorrelationId,
}

impl Default for Tracing {
    fn default() -> Self {
        Tracing {
            level: LogLevel::default(),
            format: LogFormat::default(),
            time_offset: LogTimeOffset::default(),
            target: LogOutput::default(),
            trace_log: false,
            correlation_id: CorrelationId::default(),
        }
    }
}

impl Tracing {
    pub fn init(&self) -> Result<(), String> {
        use crate::tracing;
        use crate::tracing_subscriber::{self, layer::SubscriberExt};
        use std::io::IsTerminal;

        println!("g-server: initializing tracing...");

        let make_writer = |target: &LogOutput| -> Result<
            crate::tracing_subscriber::fmt::writer::BoxMakeWriter,
            String,
        > {
            match target {
                LogOutput::StdOut => Ok(
                    crate::tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stdout),
                ),
                LogOutput::StdErr => Ok(
                    crate::tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stderr),
                ),
                LogOutput::File(path) => {
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .map_err(|err| err.to_string())?;

                    Ok(crate::tracing_subscriber::fmt::writer::BoxMakeWriter::new(
                        move || file.try_clone().expect("failed to clone tracing log file"),
                    ))
                }
            }
        };

        let make_ansi = |params: &Tracing| {
            let target = match &params.target {
                LogOutput::StdOut => std::io::stdout().is_terminal(),
                LogOutput::StdErr => std::io::stderr().is_terminal(),
                LogOutput::File(_) => false,
            };

            target && !matches!(params.format, LogFormat::Json)
        };

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
            .with_writer(make_writer(&self.target)?)
            .with_ansi(make_ansi(self))
            .with_span_events(span_events.clone())
            .with_timer(utc_timer.clone());
        let fmt_layer_utc_pretty = tracing_subscriber::fmt::layer()
            .with_writer(make_writer(&self.target)?)
            .with_ansi(make_ansi(self))
            .with_span_events(span_events.clone())
            .with_timer(utc_timer.clone())
            .pretty();
        let fmt_layer_utc_json = tracing_subscriber::fmt::layer()
            .with_writer(make_writer(&self.target)?)
            .with_ansi(make_ansi(self))
            .with_span_events(span_events.clone())
            .with_timer(utc_timer)
            .json();

        let fmt_layer_local_default = tracing_subscriber::fmt::layer()
            .with_writer(make_writer(&self.target)?)
            .with_ansi(make_ansi(self))
            .with_span_events(span_events.clone())
            .with_timer(local_timer.clone());
        let fmt_layer_local_pretty = tracing_subscriber::fmt::layer()
            .with_writer(make_writer(&self.target)?)
            .with_ansi(make_ansi(self))
            .with_span_events(span_events.clone())
            .with_timer(local_timer.clone())
            .pretty();
        let fmt_layer_local_json = tracing_subscriber::fmt::layer()
            .with_writer(make_writer(&self.target)?)
            .with_ansi(make_ansi(self))
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

    pub async fn tracing_middleware(
        crate::axum::extract::State((corr_id_cfg, req_id_header)): crate::axum::extract::State<(
            CorrelationId,
            Option<crate::http::HeaderName>,
        )>,
        request: crate::axum::extract::Request,
        next: crate::axum::middleware::Next,
    ) -> crate::axum::response::Response {
        use crate::tracing::Instrument;

        let corr_id_hdr_val = if let Some(corr_id_val) = request.headers().get(&corr_id_cfg.header)
        {
            corr_id_val
        } else {
            &corr_id_cfg
                .try_new_id()
                .unwrap_or(crate::http::HeaderValue::from_static("<invalid>"))
        };

        let span = if let Some(req_id_header) = req_id_header {
            match request.headers().get(&req_id_header) {
                Some(value) => crate::tracing::info_span!(
                    "request",
                    method = %request.method(),
                    uri = %request.uri(),
                    correlation_id = %corr_id_hdr_val.to_str().unwrap_or("<invalid>"),
                    request_id = %value.to_str().unwrap_or("<invalid>"),
                ),
                None => crate::tracing::info_span!(
                    "request",
                    method = %request.method(),
                    uri = %request.uri(),
                    correlation_id = %corr_id_hdr_val.to_str().unwrap_or("<invalid>"),
                ),
            }
        } else {
            crate::tracing::info_span!(
                "request",
                method = %request.method(),
                uri = %request.uri(),
                correlation_id = %corr_id_hdr_val.to_str().unwrap_or("<invalid>"),
            )
        };

        crate::tracing::info!("::REQUEST_BEGIN::");

        let resp = next.run(request).instrument(span).await;

        crate::tracing::info!("::REQUEST_END::");

        resp
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

#[derive(Debug, Clone, Default)]
pub enum LogOutput {
    #[default]
    StdOut,
    StdErr,
    File(std::path::PathBuf),
}

// Metrics
#[derive(Debug, Clone)]
pub struct Metrics {
    pub system_metrics: SystemMetrics,

    pub prometheus: Option<Prometheus>,

    /// `init()` function to initializes the metrics recorder
    pub init_fn: Option<
        fn()
            -> std::pin::Pin<std::boxed::Box<dyn std::future::Future<Output = Result<(), String>>>>,
    >,
}

impl Metrics {
    #[cfg(feature = "metrics")]
    pub fn init(&self) -> Result<(), String> {
        println!("g-server: initializing metrics...");
        self.init_system_metrics()?;

        Ok(())
    }

    #[cfg(feature = "metrics")]
    pub fn init_system_metrics(&self) -> Result<(), String> {
        use crate::sysinfo::{Disks, Networks, System};

        let cfg = self.system_metrics.clone();
        if !cfg.enable {
            return Ok(());
        }
        if cfg.refresh_duration.is_zero() {
            return Err("g-server: system metrics `refresh_duration` must not be zero".into());
        }

        println!("g-server: initializing system metrics...");

        tokio::spawn(async move {
            let mut sys = System::new_all();
            let mut disks = Disks::new_with_refreshed_list();
            let mut networks = Networks::new_with_refreshed_list();
            let mut cpu_reader = CpuReader::new().await;
            let iface = NetworkReader::default_interface().await;

            // tokio::time::interval panics on a zero period.
            let period = cfg
                .refresh_duration
                .max(std::time::Duration::from_millis(1));
            let mut ticker = crate::tokio::time::interval(period);
            ticker.set_missed_tick_behavior(crate::tokio::time::MissedTickBehavior::Skip);

            // CPU usage is a diff between two refreshes, so prime it once;
            // otherwise the first reading would be inaccurate.
            sys.refresh_cpu_usage();

            loop {
                ticker.tick().await;

                // CPU: sysinfo reports percent (0..100), OTel utilization is 0..1.
                // cgroup aware
                if cfg.cpu_usage.enable {
                    let cpu_usage = cpu_reader.read(&mut sys).await;
                    record(&cfg.cpu_usage, cpu_usage);
                }

                // Memory (bytes)
                // cgroup aware
                if cfg.memory_usage.enable || cfg.memory_avail.enable {
                    let (used_memory, avail_memory) = MemoryReader::read(&mut sys);
                    record(&cfg.memory_usage, used_memory);
                    record(&cfg.memory_avail, avail_memory);
                }

                // Disk (bytes)
                // host wide
                if cfg.disk_usage.enable || cfg.disk_avail.enable {
                    let (used_disk, avail_disk) = DiskReader::read(&mut disks);
                    record(&cfg.disk_usage, used_disk);
                    record(&cfg.disk_avail, avail_disk);
                }

                // Network(bytes)
                // container aware
                if cfg.network_received.enable || cfg.network_sent.enable {
                    let (network_received, network_transmitted) =
                        NetworkReader::read(&mut networks, iface.as_deref());
                    record(&cfg.network_received, network_received);
                    record(&cfg.network_sent, network_transmitted);
                }
            }
        });

        Ok(())
    }
}

#[cfg(feature = "metrics")]
fn record(cfg: &MetricsConfig, value: f64) {
    if !cfg.enable {
        return;
    }

    let labels: Vec<metrics::Label> = cfg
        .labels
        .iter()
        .map(|(k, v)| metrics::Label::new(*k, *v))
        .collect();

    match cfg.metric_type {
        MetricType::Gauge => metrics::gauge!(cfg.name, labels).set(value),
        MetricType::Counter => metrics::counter!(cfg.name, labels).increment(value as u64),
        MetricType::Histogram => metrics::histogram!(cfg.name, labels).record(value),
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            system_metrics: SystemMetrics::default(),
            prometheus: None,
            init_fn: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub enable: bool,
    pub refresh_duration: std::time::Duration,
    pub cpu_usage: MetricsConfig,
    pub memory_usage: MetricsConfig,
    pub memory_avail: MetricsConfig,
    pub disk_usage: MetricsConfig,
    pub disk_avail: MetricsConfig,
    pub network_received: MetricsConfig,
    pub network_sent: MetricsConfig,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            enable: true,
            refresh_duration: std::time::Duration::from_millis(1000), // every second
            cpu_usage: MetricsConfig {
                enable: true,
                name: "system.cpu.utilization",
                metric_type: MetricType::Gauge,
                labels: &[],
            },
            memory_usage: MetricsConfig {
                enable: true,
                name: "system.memory.usage",
                metric_type: MetricType::Gauge,
                labels: &[("system.memory.state", "used")],
            },
            memory_avail: MetricsConfig {
                enable: true,
                name: "system.memory.usage",
                metric_type: MetricType::Gauge,
                labels: &[("system.memory.state", "free")],
            },
            disk_usage: MetricsConfig {
                enable: true,
                name: "system.filesystem.usage",
                metric_type: MetricType::Gauge,
                labels: &[("system.filesystem.state", "used")],
            },
            disk_avail: MetricsConfig {
                enable: true,
                name: "system.filesystem.usage",
                metric_type: MetricType::Gauge,
                labels: &[("system.filesystem.state", "free")],
            },
            network_received: MetricsConfig {
                enable: false,
                name: "system.network.io",
                metric_type: MetricType::Counter,
                labels: &[("network.io.direction", "receive")],
            },
            network_sent: MetricsConfig {
                enable: false,
                name: "system.network.io",
                metric_type: MetricType::Counter,
                labels: &[("network.io.direction", "transmit")],
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MetricsConfig {
    pub enable: bool,
    pub name: &'static str,
    pub metric_type: MetricType,
    pub labels: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Clone, Copy)]
pub enum MetricType {
    Gauge,
    Counter,
    Histogram,
}

#[derive(Debug, Clone, Copy)]
pub struct CpuReader {
    prev: Option<(u64, Instant)>,
}

impl CpuReader {
    pub async fn new() -> Self {
        Self {
            prev: Self::cgroup_usage_usec().await.map(|u| (u, Instant::now())),
        }
    }

    async fn cgroup_usage_usec() -> Option<u64> {
        fs::read_to_string("/sys/fs/cgroup/cpu.stat")
            .await
            .ok()?
            .lines()
            .find_map(|l| l.strip_prefix("usage_usec ")?.trim().parse().ok())
    }

    /// CPUs this cgroup may use: quota from `cpu.max` if set, else all visible cores.
    async fn cgroup_cpu_limit() -> f64 {
        fs::read_to_string("/sys/fs/cgroup/cpu.max")
            .await
            .ok()
            .and_then(|s| {
                let mut it = s.split_whitespace();
                let quota: f64 = it.next()?.parse().ok()?; // "max" -> None
                let period: f64 = it.next()?.parse().ok()?;
                Some(quota / period)
            })
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(1.0, |n| n.get() as f64))
    }

    /// CPU utilization in 0..1. Relative to the container's CPU limit when
    /// cgroup v2 is available, otherwise host-wide via the given `System`.
    pub async fn read(&mut self, sys: &mut crate::sysinfo::System) -> f64 {
        if let Some(usec) = Self::cgroup_usage_usec().await {
            let now = Instant::now();
            if let Some((prev_usec, prev_at)) = self.prev.replace((usec, now)) {
                let used = usec.saturating_sub(prev_usec) as f64;
                let wall = now.duration_since(prev_at).as_micros() as f64;
                if wall > 0.0 {
                    return (used / (wall * Self::cgroup_cpu_limit().await)).clamp(0.0, 1.0);
                }
            }
        }
        sys.refresh_cpu_usage();
        f64::from(sys.global_cpu_usage()) / 100.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryReader;

impl MemoryReader {
    pub(crate) fn read(sys: &mut crate::sysinfo::System) -> (f64, f64) {
        sys.refresh_memory();
        match sys.cgroup_limits() {
            Some(l) => {
                let used = l.rss;
                let avail = l.total_memory.saturating_sub(used);
                (used as f64, avail as f64)
            }
            None => {
                let used = sys.used_memory();
                let avail = sys.total_memory().saturating_sub(used);
                (used as f64, avail as f64)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DiskReader;

impl DiskReader {
    pub(crate) fn read(disks: &mut crate::sysinfo::Disks) -> (f64, f64) {
        disks.refresh(true);
        let path = std::path::Path::new("/");
        if let Some((total, avail)) = disks
            .list()
            .iter()
            .filter(|d| path.starts_with(d.mount_point()))
            .max_by_key(|d| d.mount_point().as_os_str().len()) // longest match wins
            .map(|d| (d.total_space(), d.available_space()))
        {
            return ((total - avail) as f64, avail as f64);
        }
        (0.into(), 0.into())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkReader;

impl NetworkReader {
    pub(crate) async fn default_interface() -> Option<String> {
        let route = fs::read_to_string("/proc/net/route").await.ok()?;
        route.lines().skip(1).find_map(|l| {
            let mut f = l.split_whitespace();
            let iface = f.next()?;
            let dest = f.next()?;
            (dest == "00000000").then(|| iface.to_owned())
        })
    }

    pub(crate) fn read(networks: &mut crate::sysinfo::Networks, iface: Option<&str>) -> (f64, f64) {
        networks.refresh(true);
        networks
            .iter()
            .filter(|(name, _)| match iface {
                Some(i) => name.as_str() == i,
                // fallback: skip loopback and virtual interfaces
                None => {
                    let n = name.as_str();
                    n != "lo"
                        && !["docker", "br-", "veth", "cni", "flannel", "cali", "virbr"]
                            .iter()
                            .any(|p| n.starts_with(p))
                }
            })
            .fold((0 as f64, 0 as f64), |(r, t), (_, d)| {
                (
                    r + d.total_received() as f64,
                    t + d.total_transmitted() as f64,
                )
            })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Prometheus {
    pub enable: bool,
    pub endpoint: &'static str,
}

impl Default for Prometheus {
    fn default() -> Self {
        Self {
            enable: true,
            endpoint: "/metrics",
        }
    }
}
