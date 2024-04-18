use crate::authdb::auth_actors::{Authority, User, Users};
use anyhow::{anyhow, Context, Result};
use lms_auth::auth::{AuthError, AuthRequest, AuthResult, AuthSucc};
use lms_auth::local_crypto::hash_256;
use lms_core::app_ctx::AppContext;
use lms_core::http::response::Response;

use reqwest::{Body, Method, Request};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthDB {
    db_path: String,
    users: Users,
    app_context: Arc<AppContext>,
}

impl AuthDB {
    pub async fn init(app_context: Arc<AppContext>, db_path: String) -> Result<Self> {
        let password = &app_context.blueprint.server.auth_pw;
        let users = user_entry(&app_context, &db_path, password, None).await?;
        Ok(Self {
            db_path,
            users,
            app_context,
        })
    }
    pub async fn handle_request(&mut self, request: Response<bytes::Bytes>) -> AuthResult {
        let auth_provider = &self.app_context.blueprint.auth;
        let auth_request = AuthRequest::try_from_encrypted(&request.body, auth_provider);

        match auth_request {
            Ok(auth_request) => {
                if !auth_request.verify_sig(auth_provider) {
                    return auth_err("Signature verification failed");
                }
                if auth_request.signup_details.is_some() {
                    self.signup(auth_request).await
                } else {
                    self.login(auth_request).await
                }
            }
            Err(_) => auth_err("Unable to deserialize auth request"),
        }
    }

    async fn signup(&mut self, req: AuthRequest) -> AuthResult {
        let signup_details = req.signup_details.unwrap();
        match verify(
            &signup_details.admin_username,
            &signup_details.admin_password,
            &self.users,
        ) {
            Ok(_) => {
                let authority = Authority::from_int(signup_details.authority);
                match authority {
                    Ok(authority) => {
                        let user = User {
                            username: req.username,
                            name: signup_details.name.clone(),
                            password: req.password,
                            authority,
                        };

                        self.users.insert(user);
                        match user_entry(
                            &self.app_context,
                            &self.db_path,
                            &self.app_context.blueprint.server.auth_pw,
                            Some(self.users.clone()),
                        )
                        .await
                        {
                            Ok(users) => self.users = users,
                            Err(e) => {
                                panic!(
                                    "Unable to perform IO. Stopping the server with error: {}",
                                    e
                                );
                            }
                        };
                        let token = self.app_context.blueprint.server.token.generate_current();
                        match token {
                            Ok(token) => auth_succ(signup_details.name, token),
                            Err(_) => auth_err("Unable to generate token"),
                        }
                    }
                    Err(e) => auth_err(e.to_string()),
                }
            }
            Err(e) => auth_err(e.to_string()),
        }
    }
    async fn login(&self, req: AuthRequest) -> AuthResult {
        // TODO respond with token
        match verify(&req.username, &req.password, &self.users) {
            Ok(user) => {
                let token = self.app_context.blueprint.server.token.generate_current();
                match token {
                    Ok(token) => auth_succ(user.name, token),
                    Err(_) => auth_err("Unable to generate token"),
                }
            }
            Err(e) => auth_err(e.to_string()),
        }
    }
}

async fn user_entry<T: AsRef<str>>(
    app_context: &Arc<AppContext>,
    db_path: &str,
    password: T,
    users: Option<Users>,
) -> Result<Users> {
    let password = hash_256(password);

    if db_path.starts_with("http") {
        let url = url::Url::parse(db_path)?;
        let mut req = Request::new(Method::POST, url);

        if let Some(users) = users {
            let user = serde_json::to_string(&users)?;
            *req.body_mut() = Some(Body::from(
                json!({
                    "operation": "put_user",
                    "users": user,
                    "pw": &password
                })
                .to_string(),
            ));
        } else {
            *req.body_mut() = Some(Body::from(
                json!({
                    "operation": "get_users",
                    "pw": &password
                })
                .to_string(),
            ));
        }

        let result = app_context.runtime.http.execute(req).await?;
        let users = serde_json::from_slice::<Users>(&result.body)?;

        Ok(users)
    } else {
        let users = if let Some(users) = users {
            let users_str = serde_json::to_string(&users)?;
            let users_str = app_context.blueprint.auth.encrypt_aes(&users_str)?;
            app_context
                .runtime
                .file
                .write(db_path, users_str.as_bytes())
                .await?; // store encrypted directly

            users
        } else {
            let encrypted_users = app_context.runtime.file.read(db_path).await?;
            let users_str = app_context.blueprint.auth.decrypt_aes(encrypted_users)?;

            serde_json::from_str::<Users>(&users_str)?
        };

        Ok(users)
    }
}

fn verify(username: &str, pw: &str, users: &Users) -> Result<User> {
    let user = users.get(username).context("No such user found")?;
    if user.password.eq(pw) {
        Ok(user)
    } else {
        Err(anyhow!("Invalid password"))
    }
}

fn auth_err<T: AsRef<str>>(message: T) -> AuthResult {
    AuthResult {
        error: Some(AuthError {
            message: message.as_ref().to_string(),
        }),
        success: None,
    }
}

fn auth_succ(name: String, token: String) -> AuthResult {
    AuthResult {
        error: None,
        success: Some(AuthSucc { name, token }),
    }
}
