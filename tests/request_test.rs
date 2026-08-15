use ::g_server::http::*;
use axum::http::uri::Scheme;
use serde::Deserialize;

#[tokio::test]
async fn request_test() {
    let uri = "https://example.com/hello/world?foo=bar#section"
        .parse::<axum::http::Uri>()
        .unwrap();

    let string_body = axum::body::Body::from("hello world");

    let mut request = Request::<axum::http::Uri, axum::http::HeaderMap, axum::body::Body>::new(
        Method::Get,
        uri,
        axum::http::HeaderMap::new(),
        string_body,
    );

    // Request getters.

    assert_eq!(request.method(), &Method::Get);

    assert_eq!(request.uri().scheme(), Some(&Scheme::HTTPS));
    assert_eq!(
        request.uri().authority().map(|a| a.as_str()),
        Some("example.com")
    );
    assert_eq!(request.uri().path(), "/hello/world");
    assert_eq!(request.uri().query(), Some("foo=bar"));
    assert_eq!(request.uri().fragment(), None); // fragment only for front-end

    assert!(request.headers().is_empty());
    assert!(request.body().is_none());

    // Request mutable accessors.

    request.method_mut().clone_from(&Method::Post);

    *request.uri_mut() = "http://localhost:3000/world?x=1#top".parse().unwrap();

    request
        .headers_mut()
        .insert("x-test", "true".parse().unwrap());

    request.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );

    #[derive(Debug, Deserialize, Clone)]
    struct User {
        name: String,
        age: u32,
    }

    let json_body = axum::body::Body::from(r#"{"name":"ren","age":25}"#);

    if let Some(body) = request.body_mut() {
        *body = json_body;
    }

    let json_body = axum::body::Body::from(r#"{"name":"ren","age":25}"#);
    request.set_body(json_body);

    assert_eq!(request.method(), &Method::Post);

    assert_eq!(request.uri().scheme(), Some(&Scheme::HTTP));
    assert_eq!(
        request.uri().authority().map(|a| a.as_str()),
        Some("localhost:3000")
    );
    assert_eq!(request.uri().path(), "/world");
    assert_eq!(request.uri().query(), Some("x=1"));
    assert_eq!(request.uri().fragment(), None); // fragment only for front-end

    assert_eq!(request.headers().get("x-test").unwrap(), "true");

    assert!(request.body().is_some());

    // into_parts.

    let (method, uri, headers, body) = request.into_parts();

    assert_eq!(method, Method::Post);

    assert_eq!(uri.scheme(), Some(&Scheme::HTTP));
    assert_eq!(uri.authority().map(|a| a.as_str()), Some("localhost:3000"));
    assert_eq!(uri.path(), "/world");
    assert_eq!(uri.query(), Some("x=1"));
    assert_eq!(uri.fragment(), None); // fragment only for front-end

    assert_eq!(headers.get("x-test").unwrap(), "true");

    // let b = request.into_body().unwrap();
    let user: User = body.unwrap().to_json().await.unwrap();
    assert_eq!(user.name, "ren");
    assert_eq!(user.age, 25);

    // ---

    let string_body = axum::body::Body::from("hello world");

    let request = Request::<axum::http::Uri, axum::http::HeaderMap, axum::body::Body>::new(
        Method::Post,
        uri,
        axum::http::HeaderMap::new(),
        string_body,
    );
    assert!(request.into_body().is_some());
}
