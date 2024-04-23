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
        let actions_request = ActionsRequest::try_from_bytes(&body);
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

        let content_id = self
            .activity
            .insert(
                actions_request.group_id, // TODO add validation for invalid grp id
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

        Ok(content_id)
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
        .totp
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions_db::actions::{ActionsContent, ActionsRead, FileWrite};
    use crate::authdb::auth_actors::Users;
    use crate::blueprint::Blueprint;
    use crate::config::batch_info::BatchInfo;
    use crate::config::config_module::ConfigModule;
    use crate::config::course_info::CourseInfo;
    use crate::file_db::file_config::Metadata;
    use lms_auth::auth::AuthProvider;
    use lms_auth::local_crypto::hash_256;
    use std::path::PathBuf;

    fn app_ctx<T: AsRef<str>>(file_db: T, actions_db: T) -> Result<AppContext> {
        let mut module = ConfigModule::default();
        module.auth.aes_key = "32bytebase64encodedkey".to_string();
        module.auth.totp.totp_secret = "base32encodedkey".to_string();
        module.auth.auth_db_path = "invalid".to_string();

        module.courses.insert(
            "course1".to_string(),
            CourseInfo {
                name: "Course 1".to_string(),
                description: Some("Course 1 description".to_string()),
            },
        );
        module.courses.insert(
            "course2".to_string(),
            CourseInfo {
                name: "Course 2".to_string(),
                description: None,
            },
        );

        module.batches = vec![BatchInfo {
            id: "22BCS".to_string(),
            courses: vec!["course1".to_string(), "course2".to_string()],
        }];

        module.server.actions_db = actions_db.as_ref().to_string();
        module.server.file_db = file_db.as_ref().to_string();

        module.extensions.users = Some(Users::default());

        let totp = module.config.auth.totp.clone().into_totp()?;
        let auth = AuthProvider::init(
            module.config.auth.auth_db_path.clone(),
            totp,
            hash_256(&module.config.auth.aes_key).clone(),
        )?;
        module.extensions.auth = Some(auth);

        let blueprint = Blueprint::try_from(module)?;
        let runtime = crate::runtime::tests::init();
        let app_ctx = AppContext { blueprint, runtime };
        Ok(app_ctx)
    }

    #[tokio::test]
    async fn test_actions_db() -> Result<()> {
        let tmp_file = tempfile::NamedTempFile::new()?;
        let tmp_file_path = tmp_file.path().to_str().unwrap();

        let tmp_dir = tempfile::tempdir()?;
        let tmp_dir_path = tmp_dir.path().to_str().unwrap();

        let app_context = app_ctx(tmp_dir_path, tmp_file_path)?;
        let totp = &app_context.blueprint.server.totp;
        let token = format!("{}_{}", "username", totp.generate_current()?);
        let token = app_context.blueprint.extensions.auth.encrypt_aes(token)?;

        let actions_db = ActionsDB::init(Arc::new(app_context)).await?;

        let write = ActionsWrite {
            title: "False title".to_string(),
            description: "False desc".to_string(),
            files: None,
            end_time: None,
            reference: "notice".to_string(),
        };

        let actions_request = ActionsRequest {
            token: token.clone(),
            group_id: "2BCS_PSD".to_string(),
            read: None,
            write: Some(write),
        };
        let actions_request = actions_request.into_serrequet()?;

        let actions_result = actions_db
            .handle_request(bytes::Bytes::from(actions_request))
            .await;
        let content_id = actions_result.message.clone();
        let content_id = String::from_utf8(BASE64_STANDARD.decode(content_id)?)?;

        let actions_result = actions_result.into_hyper_response()?;
        assert_eq!(actions_result.status(), 200);
        let expected = r#"{"actions":{"2BCS_PSD":[{"is_notif":true,"content_id":"REPLACE"}]}}"#
            .replace("REPLACE", &content_id);
        assert_eq!(
            actions_db
                .app_context
                .runtime
                .file
                .read(tmp_file_path)
                .await?,
            expected
        );

        let read = ActionsRead {
            content_id: content_id.clone(),
            file_name: None,
        };

        let actions_request = ActionsRequest {
            token: token.clone(),
            group_id: "2BCS_PSD".to_string(),
            read: Some(read),
            write: None,
        };
        let actions_request = actions_request.into_serrequet()?;

        let actions_result = actions_db
            .handle_request(bytes::Bytes::from(actions_request))
            .await;
        let config = actions_result.message.clone();
        let config = String::from_utf8(BASE64_STANDARD.decode(config)?)?;
        let config = serde_json::from_str::<Metadata>(&config)?;
        assert_eq!(config.title, "False title");
        assert_eq!(config.description, "False desc");

        let actions_result = actions_result.into_hyper_response()?;
        assert_eq!(actions_result.status(), 200);

        let actions_request = ActionsRequest {
            token: token.clone(),
            group_id: "2BCS_PSD".to_string(),
            read: None,
            write: None,
        };

        let actions_request = actions_request.into_serrequet()?;

        let actions_result = actions_db
            .handle_request(bytes::Bytes::from(actions_request))
            .await;
        let actions = actions_result.message.clone();
        let actions = String::from_utf8(BASE64_STANDARD.decode(actions)?)?;
        let actions = serde_json::from_str::<Vec<ActionsContent>>(&actions)?;

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].content_id, content_id);
        assert!(actions[0].is_notif);

        let actions_result = actions_result.into_hyper_response()?;
        assert_eq!(actions_result.status(), 200);

        let write = ActionsWrite {
            title: "False title again".to_string(),
            description: "False desc again".to_string(),
            files: None,
            end_time: None,
            reference: "notice".to_string(),
        };

        let actions_request = ActionsRequest {
            token: token.clone(),
            group_id: "2BCS_OOP".to_string(),
            read: None,
            write: Some(write),
        };
        let actions_request = actions_request.into_serrequet()?;

        let actions_result = actions_db
            .handle_request(bytes::Bytes::from(actions_request))
            .await;
        let content_id_new = actions_result.message.clone();
        let content_id_new = String::from_utf8(BASE64_STANDARD.decode(content_id_new)?)?;

        let actions_result = actions_result.into_hyper_response()?;
        assert_eq!(actions_result.status(), 200);
        let expected = r#"{"actions":{"2BCS_PSD":[{"is_notif":true,"content_id":"REPLACE"}],"2BCS_OOP":[{"is_notif":true,"content_id":"REP_NEW"}]}}"#
            .replace("REPLACE", &content_id).replace("REP_NEW", &content_id_new);

        let expected = serde_json::from_str::<ActionsActivity>(&expected)?;
        let actual = serde_json::from_str::<ActionsActivity>(
            &actions_db
                .app_context
                .runtime
                .file
                .read(tmp_file_path)
                .await?,
        )?;

        let map1 = expected
            .actions
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        let map2 = actual
            .actions
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(map1, map2);

        Ok(())
    }

    #[tokio::test]
    async fn test_insert_files() -> Result<()> {
        let tmp_file = tempfile::NamedTempFile::new()?;
        let tmp_file_path = tmp_file.path().to_str().unwrap();

        let tmp_dir = tempfile::tempdir()?;
        let tmp_dir_path = tmp_dir.path().to_str().unwrap();

        let app_context = app_ctx(tmp_dir_path, tmp_file_path)?;
        let totp = &app_context.blueprint.server.totp;
        let token = format!("{}_{}", "username", totp.generate_current()?);
        let token = app_context.blueprint.extensions.auth.encrypt_aes(token)?;

        let actions_db = ActionsDB::init(Arc::new(app_context)).await?;

        let file_content = "file1 content";

        let write = ActionsWrite {
            title: "False title".to_string(),
            description: "False desc".to_string(),
            files: Some(vec![FileWrite {
                file_name: "file1".to_string(),
                content: file_content.to_string(),
            }]),
            end_time: None,
            reference: "notice".to_string(),
        };

        let actions_request = ActionsRequest {
            token: token.clone(),
            group_id: "2BCS_PSD".to_string(),
            read: None,
            write: Some(write),
        };

        let actions_request = actions_request.into_serrequet()?;

        let actions_result = actions_db
            .handle_request(bytes::Bytes::from(actions_request))
            .await;

        let content_id = actions_result.message.clone();
        let content_id = String::from_utf8(BASE64_STANDARD.decode(content_id)?)?;

        let actions_result = actions_result.into_hyper_response()?;
        assert_eq!(actions_result.status(), 200);

        let stored_content = actions_db
            .app_context
            .runtime
            .file
            .read(
                PathBuf::from(tmp_dir_path)
                    .join(content_id.clone())
                    .join("file1")
                    .to_str()
                    .unwrap(),
            )
            .await?;

        assert_eq!(file_content, stored_content);

        let read = ActionsRead {
            content_id: content_id.clone(),
            file_name: Some("file1".to_string()),
        };

        let actions_request = ActionsRequest {
            token: token.clone(),
            group_id: "2BCS_PSD".to_string(),
            read: Some(read),
            write: None,
        };
        let actions_request = actions_request.into_serrequet()?;

        let actions_result = actions_db
            .handle_request(bytes::Bytes::from(actions_request))
            .await;

        let file = actions_result.message;
        let file = String::from_utf8(BASE64_STANDARD.decode(file)?)?;
        let file = serde_json::from_str::<FileHolder>(&file)?;

        assert_eq!(file.name, "file1");
        assert_eq!(file.content, file_content);

        Ok(())
    }

    #[tokio::test]
    async fn test_verify_token() -> Result<()> {
        let app_ctx = app_ctx("invalid", "invalid")?;
        let token = format!(
            "{}_{}",
            "username",
            app_ctx.blueprint.server.totp.generate_current()?
        );
        let token = app_ctx.blueprint.extensions.auth.encrypt_aes(token)?;

        let app_ctx = Arc::new(app_ctx);

        let actions_db = ActionsDB::init(app_ctx.clone()).await?;

        let result = verify_token(&token, &actions_db.app_context);
        assert!(result.is_ok());

        let token = "invalid_token";
        let result = verify_token(token, &actions_db.app_context);
        assert!(result.is_err());

        let token = app_ctx.blueprint.server.totp.generate_current()?;
        let token = app_ctx.blueprint.extensions.auth.encrypt_aes(token)?;
        let result = verify_token(&token, &actions_db.app_context);
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_token() {
        let app_context = app_ctx("invalid", "invalid").unwrap();
        let actions_db = ActionsDB::init(Arc::new(app_context)).await.unwrap();

        let actions_request = ActionsRequest {
            token: "invalid_token".to_string(),
            group_id: "2BCS_PSD".to_string(),
            read: None,
            write: None,
        };

        let actions_request = serde_json::to_string(&actions_request).unwrap();
        /*        let actions_request = actions_db
                    .app_context
                    .blueprint
                    .extensions
                    .auth
                    .encrypt_aes(actions_request)
                    .unwrap();
        */
        let actions_result = actions_db
            .handle_request(bytes::Bytes::from(actions_request))
            .await;

        assert_eq!(actions_result.status, 500);
        let decoded_msg =
            String::from_utf8(BASE64_STANDARD.decode(actions_result.message).unwrap()).unwrap();
        assert_eq!("Unable to decrypt token", decoded_msg);
    }

    #[tokio::test]
    async fn test_invalid_content_id() {
        let app_context = app_ctx("invalid", "invalid").unwrap();
        let actions_db = ActionsDB::init(Arc::new(app_context)).await.unwrap();

        let totp = &actions_db.app_context.blueprint.server.totp;
        let token = format!("{}_{}", "username", totp.generate_current().unwrap());
        let token = actions_db
            .app_context
            .blueprint
            .extensions
            .auth
            .encrypt_aes(token)
            .unwrap();

        let actions_request = ActionsRequest {
            token: token.clone(),
            group_id: "invalid_group_id".to_string(),
            read: Some(ActionsRead {
                content_id: "content_id".to_string(),
                file_name: None,
            }),
            write: None,
        };

        let actions_request = actions_request.into_serrequet().unwrap();

        let actions_result = actions_db
            .handle_request(bytes::Bytes::from(actions_request))
            .await;

        assert_eq!(actions_result.status, 500);
        let decoded_msg =
            String::from_utf8(BASE64_STANDARD.decode(actions_result.message).unwrap()).unwrap();
        assert_eq!(
            decoded_msg,
            "File: invalid/content_id/config.json not found"
        );
    }
    // TODO add validation for invalid grp id
}
