use http::HeaderMap;

pub struct Request<PathParams = (), QueryParams = (), Body = ()> {
    pub headers: HeaderMap,
    pub path_params: PathParams,
    pub query_params: QueryParams,
    pub body: Body,
}
