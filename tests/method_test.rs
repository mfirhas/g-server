use ::g_server::http::Method;

#[test]
fn method() {
    let methods = [
        (Method::Get, "GET"),
        (Method::Post, "POST"),
        (Method::Put, "PUT"),
        (Method::Patch, "PATCH"),
        (Method::Delete, "DELETE"),
        (Method::Head, "HEAD"),
        (Method::Options, "OPTIONS"),
        (Method::Connect, "CONNECT"),
        (Method::Trace, "TRACE"),
        (Method::Query, "QUERY"),
        (Method::Any, "*"),
    ];

    for (method, expected) in methods {
        assert_eq!(method.as_str(), expected);
        assert_eq!(method.to_string(), expected);
    }

    assert!(Method::Get.is_get());
    assert!(Method::Post.is_post());
    assert!(Method::Put.is_put());
    assert!(Method::Patch.is_patch());
    assert!(Method::Delete.is_delete());
    assert!(Method::Head.is_head());
    assert!(Method::Options.is_options());
    assert!(Method::Connect.is_connect());
    assert!(Method::Trace.is_trace());
    assert!(Method::Query.is_query());

    assert!(!Method::Any.is_get());
    assert!(!Method::Any.is_post());
    assert!(!Method::Any.is_put());
    assert!(!Method::Any.is_patch());
    assert!(!Method::Any.is_delete());
    assert!(!Method::Any.is_head());
    assert!(!Method::Any.is_options());
    assert!(!Method::Any.is_connect());
    assert!(!Method::Any.is_trace());
    assert!(!Method::Any.is_query());

    // Each concrete method must only match its own predicate.
    for (method, _) in methods {
        let predicates = [
            method.is_get(),
            method.is_post(),
            method.is_put(),
            method.is_patch(),
            method.is_delete(),
            method.is_head(),
            method.is_options(),
            method.is_connect(),
            method.is_trace(),
            method.is_query(),
        ];

        if method == Method::Any {
            assert!(!predicates.iter().any(|value| *value));
        } else {
            assert_eq!(predicates.iter().filter(|value| **value).count(), 1);
        }
    }
}
