use std::path::PathBuf;
use anyhow::{anyhow, Context};
use serde_json::json;

use lms_core::runtime::TargetRuntime;
use lms_core::uid_gen::UidGenerator;

use crate::file_config::{FileHolder, InsertionInfo, LocalFileConfig, RemoteFileConfig};

const MAX_FILE_SIZE: usize = 1024 * 1024 * 10; // 10MB

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

    pub async fn insert(&self, insertion_info: InsertionInfo, files: Vec<FileHolder>) -> anyhow::Result<String> {
        validate_files(&files)?;
        let uid_generator = UidGenerator::default();
        let uid = uid_generator.generate(self.target_runtime.instance.now().map_err(|_| anyhow!("Unable to generate UID"))?);
        if self.is_url {
            let mut url = url::Url::parse(&self.db_path)?;
            url.set_path(&uid);
            let mut req = reqwest::Request::new(reqwest::Method::POST, url.clone());
            let file_config = serde_json::to_string(&RemoteFileConfig::combine_info(insertion_info, files)).map_err(|e| anyhow!("Unable to generate body for further request with err: {}",e))?;
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
            self.target_runtime.file.create_dirs(path).await.map_err(|e| anyhow!("Unable to create dir for uid: {} with err: {}", uid, e))?;

            let local_config = LocalFileConfig::combine_info(insertion_info, &files);
            for file in files {
                let path = PathBuf::from(path).join(&file.name).to_str().context("Unable to generate path1")?;
                self.target_runtime.file.write(path, file.content.as_ref()).await?;
            }
            let local_config = serde_json::to_string(&local_config)?;
            let path = PathBuf::from(path).join("config.json").to_str().context("Unable to generate path2")?;
            self.target_runtime.file.write(path, local_config.as_bytes()).await?;
        }
        Ok(uid)
    }

    pub async fn get(&self, uid: String, file_name: &str) -> anyhow::Result<FileHolder> {
        if self.is_url {
            let mut url = url::Url::parse(&self.db_path)?;
            url.set_path(&uid);
            let mut req = reqwest::Request::new(reqwest::Method::POST, url);
            *req.body_mut() = Some(reqwest::Body::from(
                json!({
                    "file_name": file_name,
                }).to_string()
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

fn validate_files(files: &Vec<FileHolder>) -> anyhow::Result<()> {
    for v in files {
        if v.content.len() > MAX_FILE_SIZE {
            return Err(anyhow!("File {} exceeds size limit", v.name));
        };
    }
    Ok(())
}
