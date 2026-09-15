#![doc = include_str!("../README.md")]

pub use ::g_server_macro::gserver;

// re-exports since it's in generated code.
pub use ::axum;
pub use ::http;
pub use ::tokio;
pub use ::tower;
pub use ::tower_http;

pub type StatusCode = ::http::StatusCode;
pub type HeaderMap = ::http::HeaderMap;

mod config;
pub use config::{Compression, Config};

mod request;
pub use request::{Request, multipart};

mod response;
pub use response::{Response, Result};

pub mod route;
pub use route::HttpMethod;

mod server;
pub use server::Server;
