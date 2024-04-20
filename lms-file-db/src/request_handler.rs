#![allow(unused)]

use anyhow::{anyhow, Context};
use serde_json::json;
use std::path::PathBuf;

use lms_core::runtime::TargetRuntime;
use lms_core::uid_gen::UidGenerator;

use crate::file_config::{FileHolder, InsertionInfo, LocalFileConfig, Metadata, RemoteFileConfig};

const MAX_FILE_SIZE: usize = 1024 * 1024 * 10; // 10MB

// TODO: Add some encryption

pub struct FileRequestHandler {
    target_runtime: TargetRuntime,
    db_path: String,
    is_url: bool,
}

impl FileRequestHandler {
    pub fn new(target_runtime: TargetRuntime, file_db_path: String) -> Self {
        Self {
            target_runtime,
            is_url: file_db_path.starts_with("http"), // assuming it's a valid url verified during config -> blueprint conversion
            db_path: file_db_path,
        }
    }

    pub async fn insert(
        &self,
        insertion_info: InsertionInfo,
        files: Vec<FileHolder>,
    ) -> anyhow::Result<String> {
        self.insert_inner(insertion_info, files, gen_uid(&self.target_runtime)?)
            .await
    }

    async fn insert_inner(
        &self,
        insertion_info: InsertionInfo,
        files: Vec<FileHolder>,
        uid: String,
    ) -> anyhow::Result<String> {
        validate_files(&files)?;
        if self.is_url {
            let mut url = url::Url::parse(&self.db_path)?;
            url.set_path(&uid);
            let mut req = reqwest::Request::new(reqwest::Method::POST, url);
            let file_config =
                serde_json::to_string(&RemoteFileConfig::combine_info(insertion_info, files))
                    .map_err(|e| {
                        anyhow!(
                            "Unable to generate body for further request with err: {}",
                            e
                        )
                    })?;
            *req.body_mut() = Some(reqwest::Body::from(file_config));

            let response = self
                .target_runtime
                .http
                .execute(req)
                .await
                .map_err(|e| anyhow!("Failed to insert into remote server with err: {}", e))?;

            if !response.status.is_success() {
                return Err(anyhow::anyhow!("Failed to insert into remote server"));
            }
        } else {
            let mut pathbuf = std::path::PathBuf::from(&self.db_path);
            pathbuf.push(&uid);
            let path = pathbuf.to_str().context("Unable to generate path")?;
            self.target_runtime
                .file
                .create_dirs(path)
                .await
                .map_err(|e| anyhow!("Unable to create dir for uid: {} with err: {}", uid, e))?;

            let local_config = LocalFileConfig::combine_info(insertion_info, &files);
            for file in files {
                let path = PathBuf::from(path).join(&file.name);
                let path = path.to_str().context("Unable to generate path1")?;

                self.target_runtime
                    .file
                    .write(path, file.content.as_ref())
                    .await?;
            }
            let local_config = serde_json::to_string(&local_config)?;
            let path = PathBuf::from(path).join("config.json");
            let path = path.to_str().context("Unable to generate path2")?;

            self.target_runtime
                .file
                .write(path, local_config.as_bytes())
                .await?;
        }
        Ok(uid)
    }

    pub async fn get_metadata(&self, uid: String) -> anyhow::Result<Metadata> {
        if self.is_url {
            let mut url = url::Url::parse(&self.db_path)?;
            url.set_path(&uid);
            let req = reqwest::Request::new(reqwest::Method::GET, url);
            let response = self.target_runtime.http.execute(req).await.map_err(|e| {
                anyhow!("Failed to get metadata from remote server with err: {}", e)
            })?;

            if !response.status.is_success() {
                return Err(anyhow::anyhow!("Failed to get metadata from remote server"));
            }

            let body = response.to_json::<RemoteFileConfig>()?.body.metadata;
            Ok(body)
        } else {
            let mut pathbuf = std::path::PathBuf::from(&self.db_path);
            pathbuf.push(&uid);
            let path = pathbuf.join("config.json");
            let path = path.to_str().context("Unable to generate path")?;
            let content = self.target_runtime.file.read(path).await?;
            let config: LocalFileConfig = serde_json::from_str(&content)?;
            Ok(config.metadata)
        }
    }

    pub async fn get(&self, uid: String, file_name: &str) -> anyhow::Result<FileHolder> {
        if self.is_url {
            let mut url = url::Url::parse(&self.db_path)?;
            url.set_path(&uid);
            let mut req = reqwest::Request::new(reqwest::Method::POST, url);
            *req.body_mut() = Some(reqwest::Body::from(
                json!({
                    "file_name": file_name,
                })
                .to_string(),
            ));
            let response = self
                .target_runtime
                .http
                .execute(req)
                .await
                .map_err(|e| anyhow!("Failed to get from remote server with err: {}", e))?;

            if !response.status.is_success() {
                return Err(anyhow::anyhow!("Failed to get from remote server"));
            }

            let body = response.to_json::<FileHolder>()?.body;
            Ok(body)
        } else {
            let mut pathbuf = std::path::PathBuf::from(&self.db_path);
            pathbuf.push(&uid);
            pathbuf.push(file_name);
            let path = pathbuf.to_str().context("Unable to generate path")?;
            let content = self.target_runtime.file.read(path).await?;
            Ok(FileHolder {
                name: file_name.to_string(),
                content: content.into_bytes(),
            })
        }
    }
}

#[inline]
fn gen_uid(target_runtime: &TargetRuntime) -> anyhow::Result<String> {
    let uid_generator = UidGenerator::default();
    let uid = uid_generator.generate(
        target_runtime
            .instance
            .now()
            .map_err(|_| anyhow!("Unable to generate UID"))?,
    );
    Ok(uid)
}

fn validate_files(files: &Vec<FileHolder>) -> anyhow::Result<()> {
    for v in files {
        if v.content.len() > MAX_FILE_SIZE {
            return Err(anyhow!("File {} exceeds size limit", v.name));
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_config::FileHolder;
    use lms_core::authdb::auth_actors::Authority;
    use std::path::PathBuf;

    fn start_mock_server() -> httpmock::MockServer {
        httpmock::MockServer::start()
    }

    #[test]
    fn test_validate_files_exceeds_size() {
        let large_content = vec![0; MAX_FILE_SIZE + 1]; // content larger than 10MB
        let file_holder = FileHolder {
            name: "large_file.txt".to_string(),
            content: large_content,
        };
        let files = vec![file_holder];
        let result = validate_files(&files);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_files_within_limit() {
        let content = vec![0; MAX_FILE_SIZE]; // exactly 10MB
        let file_holder = FileHolder {
            name: "valid_size_file.txt".to_string(),
            content,
        };
        let files = vec![file_holder];
        let result = validate_files(&files);
        assert!(result.is_ok());
    }

    // Use mockall or a similar crate to create mocks for your dependencies
    #[tokio::test]
    async fn test_insert_into_remote() {
        let req = "{\"files\":[{\"name\":\"test.txt\",\"content\":\"AQID\"}],\"metadata\":{\"title\":\"\",\"description\":\"\",\"timestamp\":0,\"authority\":\"Student\"}}";

        let rt = crate::tests::init();
        let uid = gen_uid(&rt).unwrap();

        let server = start_mock_server();
        server.mock(|w, t| {
            w.body(req)
                .path(format!("/{}", uid))
                .method(httpmock::Method::POST);
            t.status(200).body("ok");
        });

        let mut handler = FileRequestHandler::new(rt, server.base_url());

        let files = vec![FileHolder {
            name: "test.txt".to_string(),
            content: vec![1, 2, 3],
        }];
        let insertion_info = InsertionInfo {
            title: "".to_string(),
            description: "".to_string(),
            timestamp: 0,
            end_time: None,
            authority: Default::default(),
        };

        let result = handler
            .insert_inner(insertion_info, files, uid.to_string())
            .await;
        assert_eq!(result.unwrap(), uid);
    }

    #[tokio::test]
    async fn test_insert_fail_remote() {
        let mut handler =
            FileRequestHandler::new(crate::tests::init(), "http://example.com".to_string());

        let files = vec![FileHolder {
            name: "test.txt".to_string(),
            content: vec![1, 2, 3],
        }];
        let insertion_info = InsertionInfo {
            title: "".to_string(),
            description: "".to_string(),
            timestamp: 0,
            end_time: None,
            authority: Default::default(),
        };

        let result = handler.insert(insertion_info, files).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_metadata_local() {
        let tmpdir = tempfile::tempdir().unwrap();
        let name = tmpdir.path().to_str().unwrap();

        let handler = FileRequestHandler::new(crate::tests::init(), name.to_string());
        let info = InsertionInfo {
            title: "title".to_string(),
            description: "description".to_string(),
            timestamp: 1,
            end_time: None,
            authority: Default::default(),
        };
        let meta = FileHolder {
            name: "foo.txt".to_string(),
            content: vec![1, 2, 3],
        };
        let uid = handler.insert(info, vec![meta]).await.unwrap();

        let result = handler.get_metadata(uid).await;
        let md = result.unwrap();
        assert_eq!(md.title, "title");
        assert_eq!(md.description, "description");
        assert_eq!(md.timestamp, 1);
        assert_eq!(md.end_time, None);
        assert_eq!(md.authority, Authority::default());
    }
}