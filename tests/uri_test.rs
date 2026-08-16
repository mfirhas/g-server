use ::g_server::http::Uri;
use axum::http::uri::Scheme;

#[test]
fn uri() {
    let uri = "https://example.com/hello/world?foo=bar"
        .parse::<axum::http::Uri>()
        .unwrap();

    assert_eq!(uri.scheme(), Some(&Scheme::HTTPS));
    assert_eq!(uri.authority().map(|a| a.as_str()), Some("example.com"));
    assert_eq!(uri.path(), "/hello/world");
    assert_eq!(uri.query(), Some("foo=bar"));
    assert_eq!(uri.fragment(), None);

    let uri = "http://localhost:3000/world?x=1"
        .parse::<axum::http::Uri>()
        .unwrap();

    assert_eq!(uri.scheme(), Some(&Scheme::HTTP));
    assert_eq!(uri.authority().map(|a| a.as_str()), Some("localhost:3000"));
    assert_eq!(uri.path(), "/world");
    assert_eq!(uri.query(), Some("x=1"));
    assert_eq!(uri.fragment(), None);

    let uri = "/hello/world?foo=bar".parse::<axum::http::Uri>().unwrap();

    assert_eq!(uri.scheme(), None);
    assert_eq!(uri.authority(), None);
    assert_eq!(uri.path(), "/hello/world");
    assert_eq!(uri.query(), Some("foo=bar"));
    assert_eq!(uri.fragment(), None);

    let uri = "/hello/world".parse::<axum::http::Uri>().unwrap();

    assert_eq!(uri.scheme(), None);
    assert_eq!(uri.authority(), None);
    assert_eq!(uri.path(), "/hello/world");
    assert_eq!(uri.query(), None);
    assert_eq!(uri.fragment(), None);

    // Verify the default Uri::fmt() implementation.

    let uri = "https://example.com/hello/world?foo=bar"
        .parse::<axum::http::Uri>()
        .unwrap();

    let formatted = uri.display();

    assert_eq!(formatted, "https://example.com/hello/world?foo=bar");
}
