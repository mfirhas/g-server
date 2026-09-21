use crate::{HeaderMap, HttpMethod};

#[derive(Debug, Clone)]
pub struct Request<PathParams = (), QueryParams = (), Body = ()> {
    pub method: HttpMethod,
    pub headers: HeaderMap,
    pub path_params: PathParams,
    pub query_params: QueryParams,
    pub body: Body,
}

pub mod multipart {
    use super::Request;
    use crate::Bytes;

    pub type FormDataRequest<PathParams, QueryParams, NonBinaryForm> =
        Request<PathParams, QueryParams, FormData<NonBinaryForm>>;

    #[derive(Debug, Clone)]
    pub struct FormData<T> {
        pub form: Option<T>,
        pub data: Option<Vec<Data>>,
    }

    impl<T> FormData<T> {
        #[inline]
        pub fn empty() -> Self {
            Self {
                form: None,
                data: None,
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Data {
        pub name: String,
        pub filename: Option<String>,
        pub content_type: Option<String>,
        pub file: Bytes,
    }
}
