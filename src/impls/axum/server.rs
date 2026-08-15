use std::{future::Future, net::SocketAddr, pin::Pin};

use axum::serve;
use tokio::net::TcpListener;

use crate::{
    http::{Request, Response, router::Router},
    server::Server,
};

type AxumRequest = Request<axum::http::Uri, axum::http::HeaderMap, axum::body::Body>;

type AxumResponse = Response<axum::http::HeaderMap, axum::body::Body>;

type ListenerFuture = Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + Send>>;

pub struct AxumServer {
    listeners: Vec<ListenerFuture>,
}

impl AxumServer {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    pub fn listen<R, S>(
        mut self,
        addr: SocketAddr,
        router: Router<AxumRequest, AxumResponse, R, S>,
    ) -> Self
    where
        R: super::router::IntoAxumRouter<S>,
        S: Clone + Send + Sync + 'static,
    {
        let router = router.into_axum();

        let listener = Box::pin(async move {
            let listener = TcpListener::bind(addr).await?;

            serve(listener, router).await.map_err(std::io::Error::other)
        });

        self.listeners.push(listener);

        self
    }
}

impl Default for AxumServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Server for AxumServer {
    type Error = std::io::Error;

    fn run(self) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            let mut tasks = Vec::with_capacity(self.listeners.len());

            for listener in self.listeners {
                tasks.push(tokio::spawn(listener));
            }

            for task in tasks {
                task.await.map_err(std::io::Error::other)??;
            }

            Ok(())
        }
    }
}
