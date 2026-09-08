//! This runs gserver without any routes registered, so it runs without ones.
//!
//! If you access it, it always returns 404.

use g_server::gserver;

gserver! {
    http("no_routes_server", "0.0.0.0", 42069){}
}
