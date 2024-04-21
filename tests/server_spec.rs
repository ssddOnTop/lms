#[cfg(test)]
pub mod test {
    use std::borrow::Cow;
    use std::sync::Arc;
    use std::time::SystemTime;

    use anyhow::{anyhow, Result};
    use hyper::body::Bytes;
    use lms_core::http::response::Response;
    use lms_core::runtime::TargetRuntime;
    use lms_core::{EnvIO, FileIO, HttpIO, Instance};
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

        async fn create_dirs<'a>(&'a self, _path: &'a str) -> Result<()> {
            Ok(())
        }
    }
    #[derive(Clone)]
    struct TestInstance {}

    impl Instance for TestInstance {
        fn now(&self) -> Result<u128> {
            Ok(SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_millis())
        }
    }

    #[derive(Clone)]
    struct TestEnv {}

    impl EnvIO for TestEnv {
        fn get(&self, _key: &str) -> Option<Cow<'_, str>> {
            Some(Cow::Borrowed("test"))
        }
    }

    pub fn init() -> TargetRuntime {
        let http = TestHttp::init();

        let file = TestFileIO::init();
        TargetRuntime {
            http,
            file,
            env: Arc::new(TestEnv {}),
            instance: Arc::new(TestInstance {}),
        }
    }
}

#[cfg(test)]
mod server_spec {
    use lms::cli::server::Server;
    use lms_auth::auth::{AuthRequest, AuthResult, SignUpDet};
    use lms_auth::local_crypto::hash_256;
    use lms_core::authdb::auth_actors::{Authority, User};
    use lms_core::config::config_module::ConfigModule;
    use lms_core::config::reader::ConfigReader;
    use reqwest::Client;

    struct TestHttp {
        url: String,
        method: reqwest::Method,
        body: String,
    }

    async fn test_req(tests: Vec<TestHttp>, config_module: ConfigModule) {
        let mut server = Server::new(config_module);
        let server_up_receiver = server.server_up_receiver();

        tokio::spawn(async move {
            server.start().await.unwrap();
        });

        server_up_receiver
            .await
            .expect("Server did not start up correctly");

        for test in tests {
            let client = Client::new();

            let task: tokio::task::JoinHandle<Result<_, anyhow::Error>> =
                tokio::spawn(async move {
                    let mut req = reqwest::Request::new(test.method, test.url.parse()?);
                    *req.body_mut() = Some(reqwest::Body::from(test.body));

                    let response = client.execute(req).await?;
                    let response_body = response.text().await?;
                    Ok(response_body)
                });

            let response = task
                .await
                .expect("Spawned task should success")
                .expect("Request should success");
            match serde_json::from_str::<AuthResult>(&response) {
                Ok(mut response) => {
                    if let Some(succ) = response.success.as_mut() {
                        succ.token = String::new(); // can't assert totp token
                    }
                    insta::assert_snapshot!(serde_json::to_string_pretty(&response).unwrap());
                }
                Err(_) => {
                    insta::assert_snapshot!(response);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_server() -> anyhow::Result<()> {
        let runtime = crate::test::init();
        let reader = ConfigReader::init(runtime);
        let mut config_module = reader.read("tests/server/config.json").await?;
        if let Some(users) = config_module.extensions.users.as_mut() {
            users.insert(User {
                username: "admin".to_string(),
                name: "admin".to_string(),
                password: hash_256("admin"),
                authority: Authority::Admin,
                batch: None,
            });
        }
        let auth = config_module.extensions.auth.as_ref().unwrap();

        let auth_req = AuthRequest::new(
            "new",
            "notNewbie",
            Some(SignUpDet {
                name: "newbie".to_string(),
                authority: 2,
                admin_username: "admin".to_string(),
                admin_password: "admin".to_string(),
                batch: Some("22BCS".to_string()),
            }),
        )?;

        let auth_req_invalid_authority = AuthRequest::new(
            "new",
            "notNewbie",
            Some(SignUpDet {
                name: "newbie".to_string(),
                authority: 3,
                admin_username: "admin".to_string(),
                admin_password: "admin".to_string(),
                batch: Some("22BCS".to_string()),
            }),
        )?;

        let auth_req_invalid_pass = AuthRequest::new("new", "notNewbieIncorrectPass", None)?;

        let auth_req_no_such_user = AuthRequest::new("noSuchUser", "notNewbie", None)?;

        test_req(
            vec![
                TestHttp {
                    url: "http://localhost:19194/invalid".to_string(),
                    method: reqwest::Method::GET,
                    body: "".to_string(),
                },
                TestHttp {
                    url: "http://localhost:19194/auth".to_string(),
                    method: reqwest::Method::POST,
                    body: "invalid aes".to_string(),
                },
                TestHttp {
                    url: "http://localhost:19194/auth".to_string(),
                    method: reqwest::Method::POST,
                    body: auth.encrypt_aes("invalid body")?,
                },
                TestHttp {
                    url: "http://localhost:19194/auth".to_string(),
                    method: reqwest::Method::POST,
                    body: auth_req.into_encrypted_request(auth)?,
                },
                TestHttp {
                    url: "http://localhost:19194/auth".to_string(),
                    method: reqwest::Method::POST,
                    body: auth_req_invalid_authority.into_encrypted_request(auth)?,
                },
                TestHttp {
                    url: "http://localhost:19194/auth".to_string(),
                    method: reqwest::Method::POST,
                    body: auth_req_no_such_user.into_encrypted_request(auth)?,
                },
                TestHttp {
                    url: "http://localhost:19194/auth".to_string(),
                    method: reqwest::Method::POST,
                    body: auth_req_invalid_pass.into_encrypted_request(auth)?,
                },
            ],
            config_module,
        )
        .await;

        Ok(())
    }
}
