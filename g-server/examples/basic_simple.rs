use g_server::{
    Request, Response, gserver,
    http::{HeaderMap, StatusCode},
};
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

async fn put(_: (), _req: Request<(), (), ()>) -> Result<Response<()>, Response<String>> {
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("nganu", 123.into());
    Err(Response::new()
        .with_status(StatusCode::CREATED)
        .with_header(resp_headers)
        .with_text("error".into()))
}

#[derive(Deserialize)]
struct Path {
    id: u64,
}

async fn id(_: (), req: Request<Path>) -> Result<Response<String>, Response<String>> {
    Ok(Response::new().with_text(format!("{}", req.path_params.id)))
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Path2 {
    user_id: u64,
}

#[allow(dead_code)]
async fn id2(_: (), req: Request<Path2>) -> Result<Response<String>, Response<String>> {
    Ok(Response::new().with_text(format!("~ {}", req.path_params.user_id)))
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

        put: {
            endpoint: "/baby",
            handler: put,
            response_body: empty,
        }

        get: {
            endpoint: "/ping/{id}",
            path_params: Path,
            response_body: text,
            handler: id,
        }

        // conflicts
        // get: {
        //     endpoint: "/ping/{user_id}",
        //     path_params: Path2,
        //     response_body: text,
        //     handler: id2,
        // }
    }

    http("another_with_handler", "127.0.0.1", 42169) {
        get: {
            endpoint: "/ping",

        }
    }
}
