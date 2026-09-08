//! This runs gserver without any routes registered on multiple servers, so it runs without ones.
//!
//! If you access it, it always returns 404.

use g_server::gserver;

gserver! {
    http("no_routes_server", "0.0.0.0", 42069){}
    http("no_routes_server_2", "0.0.0.0", 42169){}
    http("no_routes_server_3", "127.0.0.1", 42269){}
    http("no_routes_server_4", "0.0.0.0", 42369){}
}

// error: duplicate server name
// gserver! {
//     http("no_routes_server", "0.0.0.0", 42069){}
//     http("no_routes_server", "0.0.0.0", 42169){}
//     http("no_routes_server_3", "0.0.0.0", 42269){}
//     http("no_routes_server_4", "0.0.0.0", 42369){}
// }

// error: duplicate port
// gserver! {
//     http("no_routes_server", "0.0.0.0", 42069){}
//     http("no_routes_server_2", "0.0.0.0", 42169){}
//     http("no_routes_server_3", "0.0.0.0", 42169){}
//     http("no_routes_server_4", "0.0.0.0", 42369){}
// }
