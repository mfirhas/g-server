use ::g_server::http::*;
use serde::Deserialize;

type Res = Response<axum::http::HeaderMap, axum::body::Body>;

fn header() -> axum::http::HeaderMap {
    axum::http::HeaderMap::new()
}

fn body() -> axum::body::Body {
    axum::body::Body::from(r#"{"message":"hello"}"#)
}

#[tokio::test]
async fn response_test() {
    let mut response = Res::new(Status::OK, header(), body());

    // Getters.

    assert_eq!(response.status(), &Status::OK);
    assert!(response.headers().is_empty());

    // Mutable accessors.

    *response.status_mut() = Status::CREATED;

    response
        .headers_mut()
        .insert("x-test", "true".parse().unwrap());

    let resp_body = axum::body::Body::from(r#"{"message":"updated"}"#);
    response.set_body(resp_body);

    assert_eq!(response.status(), &Status::CREATED);
    assert_eq!(response.headers().get("x-test").unwrap(), "true");

    // into_body.

    let mut response = Res::new(Status::OK, header(), body());

    #[derive(Debug, Deserialize, Clone)]
    struct User {
        message: String,
    }

    if let Some(b) = response.body_mut() {
        *b = axum::body::Body::from(r#"{"message":"mutated"}"#);
    }
    let _b = response.body().unwrap();

    let b = response.into_body().unwrap();
    let ret: User = b.to_json().await.unwrap();
    assert_eq!(ret.message, "mutated");

    // into_parts.

    let response = Res::new(Status::ACCEPTED, header(), body());

    let (status, headers, _body) = response.into_parts();

    assert_eq!(status, Status::ACCEPTED);
    assert!(headers.is_empty());

    let ret: User = _body.unwrap().to_json().await.unwrap();
    assert_eq!(ret.message, "hello");

    // IntoResponse for Response.

    let response = Res::new(Status::OK, header(), body());

    assert_eq!(response.status(), &Status::OK);
    assert!(response.body().is_some());

    // IntoResponse for Result<T, E> — Ok.

    let response = Res::new(Status::NO_CONTENT, header(), body());

    let result: Result<Res, Res> = Ok(response);

    let converted: Res = result.into_response();

    assert_eq!(converted.status(), &Status::NO_CONTENT);
    assert!(converted.body().is_none());

    // ---

    let response = Res::new(Status::NOT_MODIFIED, header(), body());

    let result: Result<Res, Res> = Ok(response);

    let converted: Res = result.into_response();

    assert_eq!(converted.status(), &Status::NOT_MODIFIED);
    assert!(converted.body().is_none());

    // informational
    let response = Res::new(Status::CONTINUE, header(), body());

    let result: Result<Res, Res> = Ok(response);

    let converted: Res = result.into_response();

    assert_eq!(converted.status(), &Status::CONTINUE);
    assert!(converted.body().is_none());

    let response = Res::new(Status::SWITCHING_PROTOCOLS, header(), body());

    let result: Result<Res, Res> = Ok(response);

    let converted: Res = result.into_response();

    assert_eq!(converted.status(), &Status::SWITCHING_PROTOCOLS);
    assert!(converted.body().is_none());

    let response = Res::new(Status::PROCESSING, header(), body());

    let result: Result<Res, Res> = Ok(response);

    let converted: Res = result.into_response();

    assert_eq!(converted.status(), &Status::PROCESSING);
    assert!(converted.body().is_none());

    let response = Res::new(Status::EARLY_HINTS, header(), body());

    let result: Result<Res, Res> = Ok(response);

    let converted: Res = result.into_response();

    assert_eq!(converted.status(), &Status::EARLY_HINTS);
    assert!(converted.body().is_none());

    // IntoResponse for Result<T, E> — Err.

    let error = Res::new(Status::BAD_REQUEST, header(), body());

    let result: Result<Res, Res> = Err(error);

    let converted: Res = result.into_response();

    assert_eq!(converted.status(), &Status::BAD_REQUEST);
    assert!(converted.body().is_some());
}
