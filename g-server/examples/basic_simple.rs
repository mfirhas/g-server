use std::fmt::Display;

use g_server::{
    Request, Response, gserver,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct Context;
impl Context {
    pub(crate) async fn init() -> std::result::Result<Self, String> {
        Ok(Self)
        // Err(String::from("sdfsdf"))
    }
}

async fn ping(_: Context, _: Request) -> Result<Response<String>, Response<String>> {
    Ok(Response::new().with_text("pong".into()))
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct UploadForm {
    name: String,
    user_id: u64,
}

async fn upload(
    _: Context,
    req: g_server::multipart::FormDataRequest<(), (), UploadForm>,
) -> Result<Response<String>, Response<String>> {
    dbg!(&req.body);
    Ok(Response::new().with_text("upload finished".into()))
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
        _: Context,
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

async fn put(_: Context, _req: Request<(), (), ()>) -> Result<Response<()>, Response<String>> {
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

async fn id(_: Context, req: Request<Path>) -> Result<Response<String>, Response<String>> {
    Ok(Response::new().with_text(format!("{}", req.path_params.id)))
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Path2 {
    user_id: u64,
}

#[allow(dead_code)]
async fn id2(_: Context, req: Request<Path2>) -> Result<Response<String>, Response<String>> {
    Ok(Response::new().with_text(format!("~ {}", req.path_params.user_id)))
}

#[allow(dead_code)]
fn root_bad_request_error() -> (StatusCode, &'static str) {
    (StatusCode::BAD_REQUEST, "bad request!!!!!!!!")
}

struct BadReq(String);
impl g_server::BadRequestErrorMessage for BadReq {
    fn bad_request_err_msg(self, err: &str) -> Self {
        BadReq(format!("{}: {}", self.0, err))
    }
}
impl Display for BadReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

gserver! {
    http("with_handler", "0.0.0.0", 42069) {
        app_context: Context,
        config: {
            normalize_endpoint: true,
            // timeout: 1000,
            body_limit: 1,
            timeout_error: text((StatusCode::GATEWAY_TIMEOUT, "you're running out of time!!"))
            concurrency_limit_error: text(Response::new().with_status(StatusCode::TOO_MANY_REQUESTS).with_text("overload!!")),
            // bad_request_error: text((StatusCode::BAD_REQUEST, BadReq("this".into()))),
            cors: {
                allowed_origins: ["https://example.com"],
                allowed_methods: [get, Post, PUT],
                allowed_headers: ["content-type", "authorization"],
                exposed_headers: ["x-request-id"],
                allow_credentials: true,
                max_age: 3600,
            },
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

        file: {
            endpoint: "/file"
            config: {
                dir: "/Users/mfirhas/github.com/mfirhas/resume/out"
                fallback_file: "READM.md"
            }
        }

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
                file: {
                    endpoint: "/file"
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
                // body_limit: 1,
                timeout: 1,
                // timeout_error: text(Response::new().with_text("asdasd".into()))
                bad_request_error: html((StatusCode::BAD_REQUEST, BadReq("<h1>BAD REQUEST</h1>".into())))
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

        post: {
            endpoint: "/upload",
            config: {
                // body_limit: 1
                // bad_request_error: html((StatusCode::BAD_REQUEST, BadReq("<h1>BAD REQUEST</h1>".into())))
            }
            request_body: form_data(UploadForm),
            handler: upload,
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
