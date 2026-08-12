use ::serde::{Serialize, de::DeserializeOwned};

#[allow(async_fn_in_trait)]
pub trait Body {
    type Error;

    async fn to_text(self) -> Result<String, Self::Error>;

    async fn to_json<T>(self) -> Result<T, Self::Error>
    where
        T: DeserializeOwned;

    fn from_text(text: String) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn from_json<T>(value: T) -> Result<Self, Self::Error>
    where
        Self: Sized,
        T: Serialize;
}
