use std::future::Future;

pub trait Server {
    type Error;

    fn run(self) -> impl Future<Output = Result<(), Self::Error>>;
}
