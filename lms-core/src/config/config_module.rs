use crate::authdb::auth_actors::Users;
use crate::config::Config;
use crate::runtime::TargetRuntime;
use hyper::Method;
use lms_auth::auth::AuthProvider;
use reqwest::{Body, Request};
use serde_json::json;
use std::ops::Deref;
#[derive(Default, Debug, Clone)]
pub struct ConfigModule {
    pub config: Config,
    pub users: Option<Users>,
}

impl Deref for ConfigModule {
    type Target = Config;
    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl From<Config> for ConfigModule {
    fn from(value: Config) -> Self {
        Self {
            config: value,
            users: None,
        }
    }
}

impl ConfigModule {
    pub async fn resolve(self, target_runtime: &TargetRuntime) -> anyhow::Result<Self> {
        let totp = self.config.auth.totp.clone().into_totp()?;

        assert!(
            self.config.auth.aes_key.len() > 8,
            "db_file_password must be at least 8 characters long"
        );

        let password = lms_auth::local_crypto::hash_256(&self.config.auth.aes_key);

        let auth = AuthProvider::init(
            self.config.auth.auth_db_path.clone(),
            totp,
            password.clone(),
        )?;

        let cfg_module = if !auth.db_path().starts_with("http") {
            match target_runtime.file.read(auth.db_path()).await {
                Ok(encrypted_users) => {
                    let decrypted_users = auth.decrypt_aes(encrypted_users)?;
                    let users = serde_json::from_str::<Users>(&decrypted_users)?;
                    ConfigModule {
                        users: Some(users),
                        ..self
                    }
                }
                Err(_) => {
                    let users = Users::default();
                    let encrypted_users = auth.encrypt_aes(serde_json::to_string(&users)?)?;
                    target_runtime
                        .file
                        .write(auth.db_path(), encrypted_users.as_bytes())
                        .await?;
                    ConfigModule {
                        users: Some(users),
                        ..self
                    }
                }
            }
        } else {
            let url = url::Url::parse(auth.db_path())?;

            let mut req = Request::new(Method::POST, url);
            *req.body_mut() = Some(Body::from(
                json!({
                    "operation": "get_users",
                    "pw": password
                })
                .to_string(),
            ));
            let result = target_runtime.http.execute(req).await?;
            let users = serde_json::from_slice::<Users>(&result.body)?;

            ConfigModule {
                users: Some(users),
                ..self
            }
        };

        Ok(cfg_module)
    }
}
