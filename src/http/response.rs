use http::{HeaderMap, StatusCode};

pub struct Response<Body = ()> {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Body,
}

pub trait IntoResponse {
    type Body;

    fn into_response(self) -> Response<Self::Body>;
}

impl<B> IntoResponse for (StatusCode, HeaderMap, B) {
    type Body = B;

    fn into_response(self) -> Response<Self::Body> {
        let (status, headers, body) = self;

        Response {
            status,
            headers,
            body,
        }
    }
}

impl IntoResponse for (StatusCode, (), ()) {
    type Body = ();

    fn into_response(self) -> Response<Self::Body> {
        let (status, _, _) = self;

        Response {
            status,
            headers: HeaderMap::new(),
            body: (),
        }
    }
}

impl IntoResponse for (StatusCode, (), String) {
    type Body = String;

    fn into_response(self) -> Response<Self::Body> {
        let (status, _, body) = self;

        Response {
            status,
            headers: HeaderMap::new(),
            body,
        }
    }
}

impl<B> IntoResponse for Response<B> {
    type Body = B;

    fn into_response(self) -> Response<Self::Body> {
        self
    }
}
