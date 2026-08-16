use ::g_server::http::Status;

#[test]
fn status() {
    // new() / as_u16() / From<u16> / Display.

    let status = Status::new(200);

    assert_eq!(status.as_u16(), 200);
    assert_eq!(status.to_string(), "200");

    let status: Status = 201.into();

    assert_eq!(status.as_u16(), 201);
    assert_eq!(status.to_string(), "201");

    // Informational.

    let informational = [
        Status::CONTINUE,
        Status::SWITCHING_PROTOCOLS,
        Status::PROCESSING,
        Status::EARLY_HINTS,
    ];

    for status in informational {
        assert!(status.is_informational());
        assert!(!status.is_success());
        assert!(!status.is_redirection());
        assert!(!status.is_client_error());
        assert!(!status.is_server_error());
    }

    // Success.

    let success = [
        Status::OK,
        Status::CREATED,
        Status::ACCEPTED,
        Status::NON_AUTHORITATIVE_INFORMATION,
        Status::NO_CONTENT,
        Status::RESET_CONTENT,
        Status::PARTIAL_CONTENT,
        Status::MULTI_STATUS,
        Status::ALREADY_REPORTED,
        Status::IM_USED,
    ];

    for status in success {
        assert!(status.is_success());
        assert!(!status.is_informational());
        assert!(!status.is_redirection());
        assert!(!status.is_client_error());
        assert!(!status.is_server_error());
    }

    // Redirection.

    let redirection = [
        Status::MULTIPLE_CHOICES,
        Status::MOVED_PERMANENTLY,
        Status::FOUND,
        Status::SEE_OTHER,
        Status::NOT_MODIFIED,
        Status::TEMPORARY_REDIRECT,
        Status::PERMANENT_REDIRECT,
    ];

    for status in redirection {
        assert!(status.is_redirection());
        assert!(!status.is_informational());
        assert!(!status.is_success());
        assert!(!status.is_client_error());
        assert!(!status.is_server_error());
    }

    // Client error.

    let client_error = [
        Status::BAD_REQUEST,
        Status::UNAUTHORIZED,
        Status::PAYMENT_REQUIRED,
        Status::FORBIDDEN,
        Status::NOT_FOUND,
        Status::METHOD_NOT_ALLOWED,
        Status::NOT_ACCEPTABLE,
        Status::PROXY_AUTHENTICATION_REQUIRED,
        Status::REQUEST_TIMEOUT,
        Status::CONFLICT,
        Status::GONE,
        Status::LENGTH_REQUIRED,
        Status::PRECONDITION_FAILED,
        Status::PAYLOAD_TOO_LARGE,
        Status::URI_TOO_LONG,
        Status::UNSUPPORTED_MEDIA_TYPE,
        Status::RANGE_NOT_SATISFIABLE,
        Status::EXPECTATION_FAILED,
        Status::IM_A_TEAPOT,
        Status::UNPROCESSABLE_CONTENT,
        Status::TOO_EARLY,
        Status::UPGRADE_REQUIRED,
        Status::TOO_MANY_REQUESTS,
        Status::REQUEST_HEADER_FIELDS_TOO_LARGE,
    ];

    for status in client_error {
        assert!(status.is_client_error());
        assert!(!status.is_informational());
        assert!(!status.is_success());
        assert!(!status.is_redirection());
        assert!(!status.is_server_error());
    }

    // Server error.

    let server_error = [
        Status::INTERNAL_SERVER_ERROR,
        Status::NOT_IMPLEMENTED,
        Status::BAD_GATEWAY,
        Status::SERVICE_UNAVAILABLE,
        Status::GATEWAY_TIMEOUT,
        Status::HTTP_VERSION_NOT_SUPPORTED,
        Status::VARIANT_ALSO_NEGOTIATES,
        Status::INSUFFICIENT_STORAGE,
        Status::LOOP_DETECTED,
        Status::NOT_EXTENDED,
        Status::NETWORK_AUTHENTICATION_REQUIRED,
    ];

    for status in server_error {
        assert!(status.is_server_error());
        assert!(!status.is_informational());
        assert!(!status.is_success());
        assert!(!status.is_redirection());
        assert!(!status.is_client_error());
    }

    // Classification boundaries.

    assert!(Status::new(100).is_informational());
    assert!(Status::new(199).is_informational());

    assert!(Status::new(200).is_success());
    assert!(Status::new(299).is_success());

    assert!(Status::new(300).is_redirection());
    assert!(Status::new(399).is_redirection());

    assert!(Status::new(400).is_client_error());
    assert!(Status::new(499).is_client_error());

    assert!(Status::new(500).is_server_error());
    assert!(Status::new(599).is_server_error());

    // Outside HTTP status classes.

    let invalid = [
        Status::new(0),
        Status::new(99),
        Status::new(600),
        Status::new(u16::MAX),
    ];

    for status in invalid {
        assert!(!status.is_informational());
        assert!(!status.is_success());
        assert!(!status.is_redirection());
        assert!(!status.is_client_error());
        assert!(!status.is_server_error());
    }
}
