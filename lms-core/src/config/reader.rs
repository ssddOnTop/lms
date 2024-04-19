#![allow(unused)]

use crate::config::config_module::ConfigModule;
use crate::config::Config;
use crate::runtime::TargetRuntime;
use reqwest::Url;
use std::path::Path;

/// Reads the configuration from a file or from an HTTP URL and resolves all linked extensions to create a ConfigModule.
pub struct ConfigReader {
    runtime: TargetRuntime,
}

/// Response of a file read operation
#[derive(Debug)]
struct FileRead {
    content: String,
    path: String,
}

impl ConfigReader {
    pub fn init(runtime: TargetRuntime) -> Self {
        Self { runtime }
    }

    /// Reads the config file and returns serialized config
    pub async fn read<T: AsRef<str>>(&self, file: T) -> anyhow::Result<ConfigModule> {
        let file = self.read_file(file).await?;
        let config = Config::from_json(&file.content)?;
        let parent = Path::new(&file.path).parent();
        let config_module = ConfigModule::from(config)
            .resolve(&self.runtime, parent)
            .await?;

        Ok(config_module)
    }
    /// Reads a file from the filesystem or from an HTTP URL
    async fn read_file<T: AsRef<str>>(&self, file: T) -> anyhow::Result<FileRead> {
        // Is an HTTP URL
        let content = if let Ok(url) = Url::parse(file.as_ref()) {
            let response = self
                .runtime
                .http
                .execute(reqwest::Request::new(reqwest::Method::GET, url))
                .await?;

            String::from_utf8(response.body.to_vec())?
        } else {
            // Is a file path

            self.runtime.file.read(file.as_ref()).await?
        };

        Ok(FileRead {
            content,
            path: file.as_ref().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    async fn get_rt() -> anyhow::Result<TargetRuntime> {
        let rt = crate::runtime::tests::init();
        let path = get_example_config();
        let content = tokio::fs::read(&path).await?;
        rt.file.write(&path, content.as_ref()).await.unwrap();
        Ok(rt)
    }

    fn start_mock_server() -> httpmock::MockServer {
        httpmock::MockServer::start()
    }

    fn get_example_config() -> String {
        let mut parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        parent.pop();

        parent
            .join("examples/config.json")
            .to_str()
            .unwrap()
            .to_string()
    }

    #[tokio::test]
    async fn test_read_file() {
        let runtime = get_rt().await.unwrap();
        let reader = ConfigReader::init(runtime);

        let example_config = get_example_config();

        let file = reader.read_file(&example_config).await.unwrap();

        assert_eq!(file.path, example_config);
    }

    #[tokio::test]
    async fn test_read_from_url() {
        let runtime = get_rt().await.unwrap();
        let reader = ConfigReader::init(runtime);
        let expected = reader.read_file(get_example_config()).await.unwrap();

        let server = start_mock_server();

        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/config.json");
            then.status(200).body(expected.content.as_str());
        });

        let actual = reader
            .read_file(format!("{}/config.json", server.base_url()))
            .await
            .unwrap();

        assert_eq!(expected.content, actual.content);
    }

    #[tokio::test]
    async fn test_read() {
        let runtime = get_rt().await.unwrap();
        let reader = ConfigReader::init(runtime);
        let example_config = get_example_config();

        let config = reader.read(example_config).await.unwrap();
        assert_eq!(config.server.port.unwrap(), 19194);
        assert_eq!(config.server.get_workers(), 4);
        assert_eq!(config.server.host.clone().unwrap(), "0.0.0.0");
        assert!(!config.auth.auth_db_path.is_empty());
        assert_eq!(config.auth.totp.totp_secret, "base32encodedkey");
        assert_eq!(config.auth.aes_key, "32bytebase64encodedkey");
    }
}
