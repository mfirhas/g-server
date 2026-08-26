#![doc = include_str!("../README.md")]

pub use ::g_server_macro::gserver;

pub use ::http;

mod config;
pub use config::{Compression, Config};

mod request;
pub use request::{Request, multipart};

mod response;
pub use response::Response;
pub use response::{IntoAxumHtmlResponse, IntoAxumJsonResponse, IntoAxumStringResponse};

pub mod route;

mod server;
pub use server::Server;
