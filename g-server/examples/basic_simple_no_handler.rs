//! Runs simple server with 2 endpoints without handlers. It returns default http unimplemented 501 handler.

use g_server::gserver;

gserver! {
    http("simple_get", "0.0.0.0", 42069) {
        get: { endpoint: "/" },
        get: {
            endpoint: "/simple",
            response_body: text,
        }
    }
}
