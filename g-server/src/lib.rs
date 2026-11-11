#![doc = include_str!("../README.md")]

pub use ::g_server_macro::gserver;

// re-exports since it's in generated code.
pub use ::axum;
pub use ::http;
pub use ::serde_urlencoded;
pub use ::tokio;
pub use ::tokio_util;
pub use ::tower;
pub use ::tower_governor;
pub use ::tower_http;
pub use ::uuid;

#[cfg(feature = "embed")]
pub use ::mime_guess;
#[cfg(feature = "embed")]
pub use ::rust_embed;

#[cfg(feature = "tls")]
pub use ::axum_server;
#[cfg(feature = "tls")]
pub use ::rustls;
#[cfg(feature = "tls")]
pub use ::rustls_pemfile;

// logging, tracing, metrics
pub use ::metrics;
pub use ::sysinfo;

#[cfg(feature = "prometheus")]
pub use ::metrics_exporter_prometheus;

pub use ::env_logger;
pub use ::log;

pub use ::tracing;
pub use ::tracing_log;
pub use ::tracing_subscriber;
// --------------------------

// OTEL
#[cfg(feature = "otel")]
pub use metrics_opentelemetry;
#[cfg(feature = "otel")]
pub use opentelemetry;
#[cfg(feature = "otel")]
pub use opentelemetry_appender_log;
#[cfg(feature = "otel")]
pub use opentelemetry_http;
#[cfg(feature = "otel")]
pub use opentelemetry_otlp;
#[cfg(feature = "otel")]
pub use opentelemetry_sdk;
#[cfg(feature = "otel")]
pub use tracing_opentelemetry;

pub type StatusCode = ::http::StatusCode;
pub type HeaderMap = ::http::HeaderMap;
pub type Bytes = ::bytes::Bytes;

pub mod config;
pub use config::{Compression, Config};

mod request;
pub use request::{Request, multipart};

mod response;
pub use response::{ErrorMessage, Response, Result};

pub mod route;
pub use route::HttpMethod;

mod server;
pub use server::Server;
