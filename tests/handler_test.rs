use ::g_server::http::router::Handler;
use ::g_server::http::{Method, Request, Response, Status};
use axum::http::HeaderMap;

type Req = Request<axum::http::Uri, axum::http::HeaderMap, axum::body::Body>;
type Res = Response<axum::http::HeaderMap, axum::body::Body>;

#[tokio::test]
async fn handler_test() {
    let handler = |_state: &(), _request: Req| async {
        Response::new(
            Status::OK,
            HeaderMap::new(),
            axum::body::Body::from("hello"),
        )
    };

    let request = Request::new(
        Method::Get,
        "/hello".parse::<axum::http::Uri>().unwrap(),
        axum::http::HeaderMap::new(),
        axum::body::Body::empty(),
    );

    let response: Res = handler.exec(&(), request).await;

    assert_eq!(response.status(), &Status::OK);
    assert!(response.headers().is_empty());
}

// fn handler(_state: &(), _request: Req) -> impl std::future::Future<Output = Res> + Send + 'static {
//     async {
//         Response::new(
//             Status::OK,
//             HeaderMap::new(),
//             axum::body::Body::from("hello"),
//         )
//     }
// }

// #[tokio::test]
// async fn handler_f_test() {
//     let request = Request::new(
//         Method::Get,
//         "/hello".parse::<axum::http::Uri>().unwrap(),
//         axum::http::HeaderMap::new(),
//         axum::body::Body::empty(),
//     );

//     let response = handler.exec(&(), request).await;

//     assert_eq!(response.status(), &Status::OK);
// }
