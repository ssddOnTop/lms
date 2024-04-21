use super::actions::{ActionsActivity, ActionsRequest, ActionsResult, ActionsWrite};
use crate::app_ctx::AppContext;
use crate::file_db::file_config::{FileHolder, InsertionInfo};
use crate::file_db::request_handler::FileRequestHandler;
use crate::runtime::TargetRuntime;
use anyhow::{anyhow, Context, Result};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use std::sync::Arc;

pub struct ActionsDB {
    app_context: Arc<AppContext>,
    file_request_handler: FileRequestHandler,
    activity: ActionsActivity,
}

impl ActionsDB {
    pub async fn init(app_context: Arc<AppContext>) -> Result<Self> {
        let actions_db_path = &app_context.blueprint.server.actions_db;

        let file_request_handler = FileRequestHandler::new(
            app_context.runtime.clone(),
            app_context.blueprint.server.file_db.clone(),
        );
        let activity = Self::fetch_activity(actions_db_path, &app_context.runtime)
            .await
            .unwrap_or_default();
        Ok(Self {
            app_context,
            file_request_handler,
            activity,
        })
    }
    async fn fetch_activity(path: &str, target_runtime: &TargetRuntime) -> Result<ActionsActivity> {
        if path.starts_with("http") {
            let url = url::Url::parse(path)?;
            let req = reqwest::Request::new(reqwest::Method::GET, url);
            let resp = target_runtime.http.execute(req).await?;
            let body = resp.body;
            Ok(serde_json::from_slice(&body)?)
        } else {
            let body = target_runtime.file.read(path).await?;
            Ok(serde_json::from_str(&body)?)
        }
    }
    pub async fn handle_request(&self, body: bytes::Bytes) -> ActionsResult {
        let auth_provider = &self.app_context.blueprint.extensions.auth;
        let actions_request = ActionsRequest::try_from_encrypted(&body, auth_provider);
        match actions_request {
            Ok(actions_request) => match verify_token(&actions_request.token, &self.app_context) {
                Ok(_) => {
                    if actions_request.write.is_some() {
                        match self.handle_write(actions_request).await {
                            Ok(msg) => actions_success(msg),
                            Err(e) => actions_error(e.to_string()),
                        }
                    } else {
                        match self.handle_read(actions_request).await {
                            Ok(msg) => actions_success(msg),
                            Err(e) => actions_error(e.to_string()),
                        }
                    }
                }
                Err(e) => actions_error(e.to_string()),
            },
            Err(e) => actions_error(e.to_string()),
        }
    }
    async fn handle_read(&self, actions_request: ActionsRequest) -> Result<String> {
        if actions_request.read.is_none() {
            let val = self
                .activity
                .get_actions(&actions_request.group_id)
                .context("Invalid group id")?;
            let data =
                serde_json::to_string(&val).map_err(|_| anyhow!("Unable to serialize data"))?;
            Ok(data)
        } else {
            let read = actions_request.read.unwrap();
            if let Some(file_name) = read.file_name {
                let file = self
                    .activity
                    .get_file_content(&read.content_id, &file_name, &self.file_request_handler)
                    .await?;
                let data = serde_json::to_string(&file)
                    .map_err(|_| anyhow!("Unable to serialize data"))?;
                Ok(data)
            } else {
                let metadata = self
                    .activity
                    .get_config(&read.content_id, &self.file_request_handler)
                    .await?;
                let data = serde_json::to_string(&metadata)
                    .map_err(|_| anyhow!("Unable to serialize data"))?;
                Ok(data)
            }
        }
    }

    async fn handle_write(&self, actions_request: ActionsRequest) -> Result<String> {
        if actions_request.write.is_none() {
            return Err(anyhow!("Invalid Actions request"));
        }
        let write = actions_request.write.unwrap();

        self.validate_write(&write).await?;

        let timestamp = self.app_context.runtime.instance.now()?;

        let info = InsertionInfo {
            title: write.title,
            description: write.description,
            timestamp,
            end_time: write.end_time,
        };

        self.activity
            .insert(
                actions_request.group_id,
                info,
                write
                    .files
                    .unwrap_or_default()
                    .into_iter()
                    .map(|file| FileHolder {
                        name: file.file_name,
                        content: file.content,
                    })
                    .collect(),
                &self.file_request_handler,
                write.reference.eq("notice"),
            )
            .await?;

        let actions_db_path = &self.app_context.blueprint.server.actions_db;
        if actions_db_path.starts_with("http") {
            let url = url::Url::parse(actions_db_path)?;
            let mut req = reqwest::Request::new(reqwest::Method::POST, url);
            *req.body_mut() = Some(reqwest::Body::from(serde_json::to_vec(&self.activity)?));

            self.app_context.runtime.http.execute(req).await?;
        } else {
            self.app_context
                .runtime
                .file
                .write(actions_db_path, &serde_json::to_vec(&self.activity)?)
                .await?;
        }

        Ok("ok".to_string())
    }
    async fn validate_write(&self, write: &ActionsWrite) -> Result<()> {
        if write.reference.is_empty() {
            return Err(anyhow!("Invalid reference"));
        }
        if write.reference != "notice" {
            let metadata = self
                .file_request_handler
                .get_metadata(&write.reference)
                .await?;
            if let Some(end_time) = metadata.end_time {
                if end_time < self.app_context.runtime.instance.now()? {
                    return Err(anyhow!("Submission time has passed"));
                }
            }
        }
        Ok(())
    }
}

fn verify_token(token: &str, app_context: &AppContext) -> Result<()> {
    let token = app_context
        .blueprint
        .extensions
        .auth
        .decrypt_aes(token)
        .map_err(|_| anyhow!("Unable to decrypt token"))?;
    let token = token.split('_').collect::<Vec<&str>>();
    if token.len() != 2 {
        return Err(anyhow!("Invalid token"));
    }
    let token = token[1];
    let token = app_context
        .blueprint
        .server
        .token
        .check_current(token)
        .map_err(|_| anyhow!("Invalid token"))?;
    if !token {
        // || username != user_name .. maybe add this in future
        return Err(anyhow!("Invalid token, please re-login"));
    }

    Ok(())
}

fn actions_error<T: AsRef<[u8]>>(message: T) -> ActionsResult {
    let message = BASE64_STANDARD.encode(message.as_ref());
    ActionsResult {
        status: 500,
        message,
    }
}

fn actions_success<T: AsRef<[u8]>>(message: T) -> ActionsResult {
    let message = BASE64_STANDARD.encode(message.as_ref());
    ActionsResult {
        status: 200,
        message,
    }
}
