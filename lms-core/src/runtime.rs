use std::sync::Arc;

use crate::{FileIO, HttpIO};

/// The TargetRuntime struct unifies the available runtime-specific
/// IO implementations. This is used to reduce piping IO structs all
/// over the codebase.
#[derive(Clone)]
pub struct TargetRuntime {
    /// HTTP client for making standard HTTP requests.
    pub http: Arc<dyn HttpIO>,
    /// Interface for file operations, tailored to the target environment's
    /// capabilities.
    pub file: Arc<dyn FileIO>,
}

#[cfg(test)]
pub mod tests {

    use std::sync::Arc;

    use anyhow::{Context, Result};
    use dashmap::DashMap;
    use hyper::body::Bytes;
    use reqwest::Client;

    use crate::http::response::Response;
    use crate::runtime::TargetRuntime;
    use crate::{FileIO, HttpIO};

    #[derive(Default)]
    struct TestHttp {
        client: Client,
    }

    impl TestHttp {
        fn init() -> Arc<Self> {
            Arc::new(Self::default())
        }
    }

    #[async_trait::async_trait]
    impl HttpIO for TestHttp {
        async fn execute(&self, request: reqwest::Request) -> Result<Response<Bytes>> {
            let response = self.client.execute(request).await;
            Response::from_reqwest(
                response?
                    .error_for_status()
                    .map_err(|err| err.without_url())?,
            )
            .await
        }
    }

    #[derive(Clone)]
    struct TestFileIO {
        hm: DashMap<String, Vec<u8>>,
    }

    impl TestFileIO {
        fn init() -> Self {
            TestFileIO { hm: DashMap::new() }
        }
    }

    #[async_trait::async_trait]
    impl FileIO for TestFileIO {
        async fn write<'a>(&'a self, path: &'a str, content: &'a [u8]) -> anyhow::Result<()> {
            self.hm.insert(path.to_string(), content.to_vec());
            Ok(())
        }

        async fn read<'a>(&'a self, path: &'a str) -> anyhow::Result<String> {
            let buffer = self
                .hm
                .get(path)
                .context(format!("File: {} not found", path))?
                .clone();
            Ok(String::from_utf8(buffer)?)
        }
    }

    pub fn init() -> TargetRuntime {
        let http = TestHttp::init();

        let file = TestFileIO::init();
        TargetRuntime {
            http,
            file: Arc::new(file),
        }
    }
}
