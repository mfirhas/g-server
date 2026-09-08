use g_server::{Request, Response, gserver, http::StatusCode};
use serde::{Deserialize, Serialize};

async fn ping(_: (), _: Request) -> Result<Response<String>, Response<String>> {
    Ok(Response::new().with_text("pong".into()))
}

#[derive(Deserialize)]
struct PostRequest {
    user_id: u64,
    user_name: String,
}

#[derive(Serialize)]
struct PostResponse {
    user_id: u64,
    user_name: String,
    message: String,
}

async fn post(
    _: (),
    req: Request<(), (), PostRequest>,
) -> Result<Response<PostResponse>, Response<String>> {
    Ok(Response::new()
        .with_status(StatusCode::CREATED)
        .with_json(PostResponse {
            user_id: req.body.user_id,
            user_name: req.body.user_name.clone(),
            message: format!("{} - {}", req.body.user_id, req.body.user_name),
        }))
}

gserver! {
    http("with_handler", "0.0.0.0", 42069) {
        get: {
            endpoint: "/ping",
            handler: ping,
            response_body: text,
        },

        post: {
            endpoint: "/post",
            request_body: json(PostRequest),
            handler: post,
        }
    }
}
