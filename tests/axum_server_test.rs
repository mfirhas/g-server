// use std::{net::SocketAddr, time::Duration};

// use g_server::{
//     http::{Body, Header, Request, Response, Status, router::Router},
//     impls::axum::server::AxumServer,
//     server::Server,
// };

// type AxumRequest = Request<axum::http::Uri, axum::http::HeaderMap, axum::body::Body>;

// type AxumResponse = Response<axum::http::HeaderMap, axum::body::Body>;

// async fn wait_for_server(url: &str) {
//     let client = reqwest::Client::new();

//     for _ in 0..50 {
//         if client.get(url).send().await.is_ok() {
//             return;
//         }

//         tokio::time::sleep(Duration::from_millis(20)).await;
//     }

//     panic!("server did not become ready: {url}");
// }

// #[tokio::test]
// async fn axum_servers_end_to_end() {
//     // ---------------------------------------------------------------------
//     // Server 1
//     // ---------------------------------------------------------------------

//     let router1: Router<AxumRequest, AxumResponse> =
//         Router::new().get("/hello", |_state: &(), _request: AxumRequest| async {
//             let mut headers = axum::http::HeaderMap::new();

//             headers.insert("x-server", axum::http::HeaderValue::from_static("one"));

//             Response::new(
//                 Status::Ok,
//                 headers,
//                 axum::body::Body::from("hello from server one"),
//             )
//         });

//     let server1 =
//         AxumServer::new().listen("127.0.0.1:3100".parse::<SocketAddr>().unwrap(), router1);

//     // ---------------------------------------------------------------------
//     // Server 2
//     // ---------------------------------------------------------------------

//     let router2: Router<AxumRequest, AxumResponse> =
//         Router::new().get("/hello", |_state: &(), _request: AxumRequest| async {
//             let mut headers = axum::http::HeaderMap::new();

//             headers.insert("x-server", axum::http::HeaderValue::from_static("two"));

//             Response::new(
//                 Status::Ok,
//                 headers,
//                 axum::body::Body::from("hello from server two"),
//             )
//         });

//     let server2 =
//         AxumServer::new().listen("127.0.0.1:3101".parse::<SocketAddr>().unwrap(), router2);

//     // ---------------------------------------------------------------------
//     // Run both servers concurrently.
//     // ---------------------------------------------------------------------

//     let task1 = tokio::spawn(async move {
//         server1.run().await.unwrap();
//     });

//     let task2 = tokio::spawn(async move {
//         server2.run().await.unwrap();
//     });

//     wait_for_server("http://127.0.0.1:3100/hello").await;
//     wait_for_server("http://127.0.0.1:3101/hello").await;

//     let client = reqwest::Client::new();

//     // ---------------------------------------------------------------------
//     // Server 1 request -> g-server Request -> Handler
//     // -> g-server Response -> Axum Response
//     // ---------------------------------------------------------------------

//     let response1 = client
//         .get("http://127.0.0.1:3100/hello")
//         .send()
//         .await
//         .unwrap();

//     assert_eq!(response1.status(), reqwest::StatusCode::OK);
//     assert_eq!(response1.headers().get("x-server").unwrap(), "one");
//     assert_eq!(response1.text().await.unwrap(), "hello from server one");

//     // ---------------------------------------------------------------------
//     // Server 2
//     // ---------------------------------------------------------------------

//     let response2 = client
//         .get("http://127.0.0.1:3101/hello")
//         .send()
//         .await
//         .unwrap();

//     assert_eq!(response2.status(), reqwest::StatusCode::OK);
//     assert_eq!(response2.headers().get("x-server").unwrap(), "two");
//     assert_eq!(response2.text().await.unwrap(), "hello from server two");

//     // ---------------------------------------------------------------------
//     // Prove the routers are actually independent.
//     // ---------------------------------------------------------------------

//     let response1_wrong = client
//         .get("http://127.0.0.1:3100/not-found")
//         .send()
//         .await
//         .unwrap();

//     assert_eq!(response1_wrong.status(), reqwest::StatusCode::NOT_FOUND);

//     let response2_wrong = client
//         .get("http://127.0.0.1:3101/not-found")
//         .send()
//         .await
//         .unwrap();

//     assert_eq!(response2_wrong.status(), reqwest::StatusCode::NOT_FOUND);

//     // The servers run forever, so stop them after the test.
//     task1.abort();
//     task2.abort();
// }
