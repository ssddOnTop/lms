use crate::actions::{ActionsActivity, ActionsRequest, ActionsResult};
use anyhow::{anyhow, Result};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use lms_core::app_ctx::AppContext;
use std::sync::Arc;
use lms_file_db::file_config::{FileHolder, InsertionInfo};
use lms_file_db::request_handler::FileRequestHandler;

pub struct ActionsDB {
    app_context: Arc<AppContext>,
    file_request_handler: FileRequestHandler,
    activity: ActionsActivity,
}

impl ActionsDB {
    pub fn init(app_context: Arc<AppContext>, activity: ActionsActivity) -> Self {
        let file_request_handler = FileRequestHandler::new(app_context.runtime.clone(), app_context.blueprint.server.file_db.clone());
        Self {
            app_context,
            file_request_handler,
            activity,
        }
    }
    pub async fn handle_request(&self, body: bytes::Bytes) -> ActionsResult {
        let auth_provider = &self.app_context.blueprint.extensions.auth;
        let actions_request = ActionsRequest::try_from_encrypted(&body, auth_provider);
        match actions_request {
            Ok(actions_request) => match verify_token(&actions_request.token, &self.app_context) {
                Ok(_) => {
                    if actions_request.read.is_some() {
                        self.handle_read(actions_request).await
                    } else if actions_request.write.is_some() {
                        self.handle_write(actions_request).await
                    } else {
                        actions_error("Invalid Actions request")
                    }
                }
                Err(e) => actions_error(e.to_string()),
            },
            Err(e) => actions_error(e.to_string()),
        }
    }
    async fn handle_read(&self, actions_request: ActionsRequest) -> ActionsResult {
        if actions_request.read.is_none() {
            return actions_error("Invalid Actions request");
        }
        let _read = actions_request.read.unwrap();
        todo!()
    }

    async fn handle_write(&self, actions_request: ActionsRequest) -> ActionsResult {
        if actions_request.write.is_none() {
            return actions_error("Invalid Actions request");
        }
        let write = actions_request.write.unwrap();
        validate_write(&write, &actions_request.group_id)?;
        let info = InsertionInfo {
            title: write.title,
            description: write.description,
            timestamp: self.app_context.runtime.instance.now()?,
            end_time: write.end_time,
            authority: Default::default(),
        };
        let result = self.activity.insert(
            actions_request.group_id,
            info,
            write.files.unwrap_or_default().into_iter().map(|file| FileHolder {
                name: file.file_name,
                content: file.content,
            }).collect(),
            &self.file_request_handler,
        ).await?;

        todo!()
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
