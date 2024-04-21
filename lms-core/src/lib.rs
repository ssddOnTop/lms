#![allow(clippy::module_inception)]
#![allow(clippy::mutable_key_type)]

use std::borrow::Cow;

pub mod actions_db;
pub mod app_ctx;
pub mod authdb;
pub mod blueprint;
pub mod config;
pub mod file_db;
pub mod http;
pub mod runtime;
pub mod uid_gen;

pub fn is_default<T: Default + Eq>(val: &T) -> bool {
    *val == T::default()
}

pub trait EnvIO: Send + Sync + 'static {
    fn get(&self, key: &str) -> Option<Cow<'_, str>>;
}

pub trait Instance: Send + Sync {
    fn now(&self) -> anyhow::Result<u128>;
}

#[async_trait::async_trait]
pub trait HttpIO: Sync + Send + 'static {
    async fn execute(
        &self,
        request: reqwest::Request,
    ) -> anyhow::Result<http::response::Response<bytes::Bytes>>;
}

#[async_trait::async_trait]
pub trait FileIO: Send + Sync {
    async fn write<'a>(&'a self, path: &'a str, content: &'a [u8]) -> anyhow::Result<()>;
    async fn read<'a>(&'a self, path: &'a str) -> anyhow::Result<String>;
    async fn create_dirs<'a>(&'a self, path: &'a str) -> anyhow::Result<()>;
}
