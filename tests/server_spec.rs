#[cfg(test)]
pub mod test {
    use std::sync::Arc;

    use anyhow::{anyhow, Result};
    use hyper::body::Bytes;
    use lms_core::http::response::Response;
    use lms_core::runtime::TargetRuntime;
    use lms_core::{FileIO, HttpIO};
    use reqwest::Client;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Clone)]
    struct TestHttp {
        client: Client,
    }

    impl Default for TestHttp {
        fn default() -> Self {
            Self {
                client: Client::new(),
            }
        }
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
    struct TestFileIO {}

    impl TestFileIO {
        fn init() -> Arc<Self> {
            Arc::new(Self {})
        }
    }

    #[async_trait::async_trait]
    impl FileIO for TestFileIO {
        async fn write<'a>(&'a self, path: &'a str, content: &'a [u8]) -> anyhow::Result<()> {
            let mut file = tokio::fs::File::create(path).await?;
            file.write_all(content)
                .await
                .map_err(|e| anyhow!("{}", e))?;
            Ok(())
        }

        async fn read<'a>(&'a self, path: &'a str) -> anyhow::Result<String> {
            let mut file = tokio::fs::File::open(path).await?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .await
                .map_err(|e| anyhow!("{}", e))?;
            Ok(String::from_utf8(buffer)?)
        }
    }

    pub fn init() -> TargetRuntime {
        let http = TestHttp::init();

        let file = TestFileIO::init();
        TargetRuntime { http, file }
    }
}

#[cfg(test)]
mod server_spec {
    use lms::cli::server::Server;
    use lms_core::config::reader::ConfigReader;
    use reqwest::Client;

    #[tokio::test]
    async fn invalid_route() {
        let url = "http://localhost:19194/invalid";

        let runtime = crate::test::init();
        let reader = ConfigReader::init(runtime);
        let config = reader
            .read("tests/server/config_notfound.json")
            .await
            .unwrap();
        let mut server = Server::new(config);
        let server_up_receiver = server.server_up_receiver();

        tokio::spawn(async move {
            server.start().await.unwrap();
        });

        server_up_receiver
            .await
            .expect("Server did not start up correctly");

        let client = Client::new();

        let task: tokio::task::JoinHandle<Result<_, anyhow::Error>> = tokio::spawn(async move {
            let response = client.get(url).send().await?;
            let response_body = response.text().await?;
            Ok(response_body)
        });

        let response = task
            .await
            .expect("Spawned task should success")
            .expect("Request should success");
        insta::assert_snapshot!(response);
    }

    #[tokio::test]
    async fn unsupported_protocol() {
        let url = "http://localhost:19194/invalid";

        let runtime = crate::test::init();
        let reader = ConfigReader::init(runtime);
        let config = reader
            .read("tests/server/config_unsupported_protocol.json")
            .await
            .unwrap();
        let mut server = Server::new(config);
        let server_up_receiver = server.server_up_receiver();

        tokio::spawn(async move {
            server.start().await.unwrap();
        });

        server_up_receiver
            .await
            .expect("Server did not start up correctly");

        let client = Client::new();

        let task: tokio::task::JoinHandle<Result<_, anyhow::Error>> = tokio::spawn(async move {
            let response = client.put(url).send().await?;
            let response_body = response.text().await?;
            Ok(response_body)
        });

        let response = task
            .await
            .expect("Spawned task should success")
            .expect("Request should success");
        insta::assert_snapshot!(response);
    }
}
