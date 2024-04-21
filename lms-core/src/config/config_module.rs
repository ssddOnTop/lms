use crate::authdb::auth_actors::Users;
use crate::config::Config;
use crate::runtime::TargetRuntime;
use hyper::Method;
use lms_auth::auth::AuthProvider;
use reqwest::{Body, Request};
use serde_json::json;
use std::ops::{Deref, DerefMut};
use std::path::Path;

#[derive(Default, Debug, Clone)]
pub struct ConfigModule {
    pub config: Config,
    pub extensions: Extensions,
}

#[derive(Default, Debug, Clone)]
pub struct Extensions {
    pub users: Option<Users>,
    pub auth: Option<AuthProvider>,
}

impl Deref for ConfigModule {
    type Target = Config;
    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl DerefMut for ConfigModule {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.config
    }
}

impl From<Config> for ConfigModule {
    fn from(value: Config) -> Self {
        Self {
            config: value,
            extensions: Extensions {
                users: None,
                auth: None,
            },
        }
    }
}

impl ConfigModule {
    pub async fn resolve(
        mut self,
        target_runtime: &TargetRuntime,
        parent_dir: Option<&Path>,
    ) -> anyhow::Result<Self> {
        let totp = self.config.auth.totp.clone().into_totp()?;
        if self.config.auth.aes_key.len() < 9 {
            anyhow::bail!("authDbPath must be at least 8 characters long");
        }

        self.config.server.actions_db =
            ConfigModule::resolve_path(&self.config.server.actions_db, parent_dir);

        self.config.auth.auth_db_path =
            ConfigModule::resolve_path(&self.config.auth.auth_db_path, parent_dir);

        let auth = AuthProvider::init(
            self.config.auth.auth_db_path.clone(),
            totp,
            lms_auth::local_crypto::hash_256(&self.config.auth.aes_key).clone(),
        )?;

        let users = if !auth.db_path().starts_with("http") {
            match target_runtime.file.read(auth.db_path()).await {
                Ok(encrypted_users) => {
                    let decrypted_users = auth.decrypt_aes(encrypted_users)?;

                    serde_json::from_str::<Users>(&decrypted_users)?
                }
                Err(_) => {
                    let users = Users::default();
                    let encrypted_users = auth.encrypt_aes(serde_json::to_string(&users)?)?;
                    target_runtime
                        .file
                        .write(auth.db_path(), encrypted_users.as_bytes())
                        .await?;
                    users
                }
            }
        } else {
            let url = url::Url::parse(auth.db_path())?;

            let mut req = Request::new(Method::POST, url);
            let password = String::from_utf8(auth.get_pw().to_vec())?;
            *req.body_mut() = Some(Body::from(
                json!({
                    "operation": "get_users",
                    "pw": &password
                })
                .to_string(),
            ));
            let result = target_runtime.http.execute(req).await?;
            serde_json::from_slice::<Users>(&result.body)?
        };

        Ok(ConfigModule {
            extensions: Extensions {
                users: Some(users),
                auth: Some(auth),
            },
            ..self
        })
    }
    fn resolve_path(src: &str, root_dir: Option<&Path>) -> String {
        if src.starts_with("http") {
            return src.to_string();
        }
        if Path::new(&src).is_absolute() {
            src.to_string()
        } else {
            let path = root_dir.unwrap_or(Path::new(""));
            path.join(src).to_string_lossy().to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::config_module::ConfigModule;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_relative_path() {
        let path_dir = Path::new("abc/xyz");
        let file_relative = "foo/bar/my.proto";
        let file_absolute = "/foo/bar/my.proto";
        assert_eq!(
            path_dir.to_path_buf().join(file_relative),
            PathBuf::from(ConfigModule::resolve_path(file_relative, Some(path_dir)))
        );
        assert_eq!(
            "/foo/bar/my.proto",
            ConfigModule::resolve_path(file_absolute, Some(path_dir))
        );
    }
}
