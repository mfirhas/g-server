#![doc = include_str!("../README.md")]

pub use ::http;

mod config;
pub use config::{Compression, Config};

mod request;
pub use request::{Request, multipart};

mod response;
pub use response::Response;
pub use response::{IntoAxumHtmlResponse, IntoAxumJsonResponse, IntoAxumStringResponse};

pub mod route;
