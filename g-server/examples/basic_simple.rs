use g_server::{
    Request, Response, gserver,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

async fn ping(_: (), _: Request) -> Result<Response<String>, Response<String>> {
    Ok(Response::new().with_text("pong".into()))
}

mod p {
    use super::*;
    #[derive(Debug, Deserialize)]
    pub struct PostRequest {
        user_id: u64,
        user_name: String,
    }

    #[derive(Serialize)]
    pub struct PostResponse {
        user_id: u64,
        user_name: String,
        message: String,
    }
    pub async fn post(
        _: (),
        req: Request<(), (), PostRequest>,
    ) -> Result<Response<PostResponse>, Response<String>> {
        dbg!(&req);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(Response::new()
            .with_status(StatusCode::CREATED)
            .with_json(PostResponse {
                user_id: req.body.user_id,
                user_name: req.body.user_name.clone(),
                message: format!("{} - {}", req.body.user_id, req.body.user_name),
            }))
    }
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

#[allow(dead_code)]
fn root_bad_request_error() -> (StatusCode, &'static str) {
    (StatusCode::BAD_REQUEST, "bad request!!!!!!!!")
}

gserver! {
    http("with_handler", "0.0.0.0", 42069) {
        config: {
            normalize_endpoint: true,
            // timeout: 1000,
            timeout_error: text((StatusCode::GATEWAY_TIMEOUT, "you're running out of time!!"))
            concurrency_limit_error: text(Response::new().with_status(StatusCode::TOO_MANY_REQUESTS).with_text("overload!!")),
            // bad_request_error: text(root_bad_request_error()),
        }
        get: {
            endpoint: "/",
            response_body: text,
            handler: |_, _| g_server::Result::<_,String>::Ok((StatusCode::OK, "OK").into()),
        }

        get: {
            endpoint: "/ping",
            handler: ping,
            response_body: text,
        },

        group: {
            prefix: "/v1",
            config: {
                concurrency_limit: 1,
                concurrency_limit_error: text(Response::new().with_status(StatusCode::TOO_MANY_REQUESTS).with_text("penuh!!")),
                timeout: 10,
                // timeout_error: text(Response::new().with_text("babi".into()))
            },
            members: [
                get: {
                    config: {
                        concurrency_limit: 0,
                        // concurrency_limit_error: html((StatusCode::TOO_MANY_REQUESTS, "<h1>OVERLOAD.........!!!!</h1>")),
                    }
                    endpoint: "/test",
                    request_body: json(p::PostRequest),
                    handler: p::post,
                }
                get: {
                    endpoint: "/wer",
                    request_body: json(p::PostRequest),
                    handler: p::post,
                }
                any: {
                    endpoint: "/post",
                    request_body: json(p::PostRequest),
                    handler: p::post,
                }
            ],
        },

        post: {
            endpoint: "/post",
            config: {
                timeout: 1,
                // timeout_error: text(Response::new().with_text("asdasd".into()))
                bad_request_error: html((StatusCode::BAD_REQUEST, "<h1>BAD REQUEST</h1>"))
            }
            request_body: json(p::PostRequest),
            handler: p::post,
        }

        get: {
            endpoint: "/test",
            request_body: json(p::PostRequest),
            handler: p::post,
        }

        get: {
            endpoint: "/zxc",
            request_body: json(p::PostRequest),
            handler: p::post,
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
