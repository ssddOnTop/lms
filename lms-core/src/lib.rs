#![allow(clippy::module_inception)]
#![allow(clippy::mutable_key_type)]

pub mod config;
pub mod runtime;
pub mod http;

#[async_trait::async_trait]
pub trait HttpIO: Sync + Send + 'static {
    async fn execute(
        &self,
        request: reqwest::Request,
    ) -> anyhow::Result<http::response::Response<bytes::Bytes>>;
}

#[async_trait::async_trait]
pub trait FileIO: Send + Sync {
    async fn write<'a>(
        &'a self,
        path: &'a str,
        content: &'a [u8],
    ) -> anyhow::Result<()>;
    async fn read<'a>(&'a self, path: &'a str) -> anyhow::Result<String>;
}