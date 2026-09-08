use crate::http::HeaderMap;

#[derive(Debug, Clone)]
pub struct Request<PathParams = (), QueryParams = (), Body = ()> {
    pub headers: HeaderMap,
    pub path_params: PathParams,
    pub query_params: QueryParams,
    pub body: Body,
}

pub mod multipart {
    #[derive(Debug)]
    pub struct Multipart {
        pub fields: Vec<MultipartField>,
    }

    #[derive(Debug)]
    pub struct MultipartField {
        pub name: Option<String>,
        pub filename: Option<String>,
        pub content_type: Option<String>,
        pub value: MultipartValue,
    }

    #[derive(Debug)]
    pub enum MultipartValue {
        Text(String),
        File(Vec<u8>),
    }
}
