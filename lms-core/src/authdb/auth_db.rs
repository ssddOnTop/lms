use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use reqwest::{Body, Method, Request};
use serde_json::json;

use lms_auth::auth::{AuthError, AuthRequest, AuthResult, AuthSucc};

use crate::app_ctx::AppContext;
use crate::authdb::auth_actors::{Authority, User, Users};

#[derive(Clone)]
pub struct AuthDB {
    users: Users,
    app_context: Arc<AppContext>,
}

impl AuthDB {
    pub async fn init(app_context: Arc<AppContext>) -> Result<Self> {
        let users = app_context.blueprint.extensions.users.clone();
        Ok(Self { users, app_context })
    }
    pub async fn handle_request(&mut self, body: bytes::Bytes) -> AuthResult {
        let auth_provider = &self.app_context.blueprint.extensions.auth;
        let auth_request = AuthRequest::try_from_encrypted(&body, auth_provider);

        match auth_request {
            Ok(auth_request) => {
                if auth_request.signup_details.is_some() {
                    self.signup(auth_request).await
                } else {
                    self.login(auth_request).await
                }
            }
            Err(e) => auth_err(e.to_string()),
        }
    }

    async fn signup(&mut self, req: AuthRequest) -> AuthResult {
        let signup_details = req.signup_details;

        let signup_details = match signup_details {
            Some(signup_details) => signup_details,
            None => return auth_err("No necessary signup details found"),
        };

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
                            batch: signup_details.batch,
                        };

                        let token = gen_token(&user.username, self.app_context.deref());

                        self.users.insert(user);
                        match user_entry(self.app_context.deref(), self.users.clone()).await {
                            Ok(users) => self.users = users,
                            Err(e) => {
                                return auth_err(format!("Unable to register user: {}", e));
                            }
                        };
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
            Ok(user) => match gen_token(&user.username, self.app_context.deref()) {
                Ok(token) => auth_succ(user.name, token),
                Err(_) => auth_err("Unable to generate token"),
            },
            Err(e) => auth_err(e.to_string()),
        }
    }
}

fn gen_token(username: &str, app_context: &AppContext) -> Result<String> {
    let token = app_context
        .blueprint
        .server
        .totp
        .generate_current()
        .map_err(|_| anyhow!("Unable to generate token"))?;
    let format = format!("{}_{}", username, token);
    let token = app_context
        .blueprint
        .extensions
        .auth
        .encrypt_aes(format)
        .map_err(|_| anyhow!("Unable to generate token, encryption err"))?;
    Ok(token)
}

pub async fn user_entry(app_context: &AppContext, users: Users) -> Result<Users> {
    let password = String::from_utf8(app_context.blueprint.extensions.auth.get_pw().to_vec())?;

    let db_path = app_context.blueprint.extensions.auth.db_path();

    if db_path.starts_with("http") {
        let url = url::Url::parse(db_path)?;
        let mut req = Request::new(Method::POST, url);

        let users = serde_json::to_string(&users)?;
        // TODO (optional) encrypt users before sending
        let resp = json!({
            "operation": "put_user",
            "users": users,
            "pw": &password
        })
        .to_string();

        *req.body_mut() = Some(Body::from(resp));

        let result = app_context.runtime.http.execute(req).await?;
        let users = serde_json::from_slice::<Users>(&result.body)?;

        Ok(users)
    } else {
        let users_str = serde_json::to_string(&users)?;
        let users_str = app_context
            .blueprint
            .extensions
            .auth
            .encrypt_aes(&users_str)?;
        app_context
            .runtime
            .file
            .write(db_path, users_str.as_bytes())
            .await?; // store encrypted directly

        Ok(users)
    }
}

fn verify(username: &str, pw: &str, users: &Users) -> Result<User> {
    let user = users.get(username).context("No such user found")?;
    if user.password.eq(pw) {
        Ok(user)
    } else {
        Err(anyhow!("Invalid password for user: {}", username))
    }
}

fn auth_err<T: AsRef<str>>(message: T) -> AuthResult {
    AuthResult {
        error: Some(AuthError {
            message: message.as_ref().to_string(),
        }),
        success: None,
        code: 500,
    }
}

fn auth_succ(name: String, token: String) -> AuthResult {
    AuthResult {
        error: None,
        success: Some(AuthSucc { name, token }),
        code: 200,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use lms_auth::auth::{AuthProvider, AuthRequest, SignUpDet};
    use lms_auth::local_crypto::hash_256;

    use crate::app_ctx::AppContext;
    use crate::authdb::auth_actors::{Authority, User, Users};
    use crate::authdb::auth_db::{user_entry, AuthDB};
    use crate::blueprint::Blueprint;
    use crate::config::config_module::ConfigModule;

    fn start_mock_server() -> httpmock::MockServer {
        httpmock::MockServer::start()
    }

    fn app_ctx<T: AsRef<str>>(db_path: T) -> anyhow::Result<AppContext> {
        let mut module = ConfigModule::default();
        module.server.actions_db = "invalid".to_string();
        module.server.file_db = "invalid".to_string();

        module.auth.aes_key = "32bytebase64encodedkey".to_string();
        module.auth.totp.totp_secret = "base32encodedkey".to_string();
        module.auth.auth_db_path = db_path.as_ref().to_string();
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

    async fn get_db() -> anyhow::Result<AuthDB> {
        let app_ctx = Arc::new(app_ctx("foobar")?);
        let auth_db = AuthDB::init(app_ctx).await?;
        Ok(auth_db)
    }

    #[tokio::test]
    async fn test_signup() -> anyhow::Result<()> {
        let mut auth_db = get_db().await?;
        let admin = User {
            username: "admin".to_string(),
            name: "admin".to_string(),
            password: hash_256("admin"),
            authority: Authority::Admin,
            batch: None,
        };
        auth_db.users.insert(admin);
        let signup = SignUpDet {
            name: "newbie".to_string(),
            authority: 2,                        // is student
            admin_username: "admin".to_string(), // siged by: admin
            admin_password: "admin".to_string(),
            batch: Some("22BCS".to_string()),
        };

        let auth_req = AuthRequest::new("new", "bie", Some(signup))?;
        let result = auth_db.signup(auth_req).await;

        assert!(result.success.is_some());
        let succ = result.success.unwrap();
        assert_eq!(succ.name, "newbie");
        Ok(())
    }

    #[tokio::test]
    async fn test_signup_fail() -> anyhow::Result<()> {
        let mut auth_db = get_db().await?;
        let auth_req = AuthRequest::new("new", "bie", None)?;

        let result = auth_db.signup(auth_req).await;
        assert!(result.error.is_some());
        let err = result.error.unwrap();
        assert_eq!(err.message, "No necessary signup details found");
        Ok(())
    }

    #[tokio::test]
    async fn test_user_entry_url() -> anyhow::Result<()> {
        let server = start_mock_server();
        server.mock(|w, t| {
            w.method(httpmock::Method::POST)
                .path("/")
                .body(r#"{"operation":"put_user","pw":"db1a2fca194f1034c91ea5d6a2df0c1a","users":"{\"users\":{}}"}"#)
            ;
            t.status(200).body(r#"{"users":{}}"#);
        });
        let app_ctx = app_ctx(server.base_url())?;
        let expected = Users::default();
        let actual = user_entry(&app_ctx, expected.clone()).await?;

        assert_eq!(expected, actual);
        Ok(())
    }

    #[tokio::test]
    async fn test_login() -> anyhow::Result<()> {
        let mut auth_db = get_db().await?;

        let newbie = User {
            username: "newbie".to_string(),
            name: "newbie".to_string(),
            password: hash_256("newbie"),
            authority: Authority::Student,
            batch: Some("22BCS".to_string()),
        };
        auth_db.users.insert(newbie);

        let auth_req = AuthRequest::new("newbie", "newbie", None)?;
        let result = auth_db.login(auth_req).await;
        assert!(result.success.is_some());
        let succ = result.success.unwrap();
        assert_eq!(succ.name, "newbie");
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_req_login() -> anyhow::Result<()> {
        let mut auth_db = get_db().await?;

        let newbie = User {
            username: "newbie".to_string(),
            name: "newbie".to_string(),
            password: hash_256("newbie"),
            authority: Authority::Student,
            batch: Some("22BCS".to_string()),
        };
        auth_db.users.insert(newbie);

        let auth_req = AuthRequest::new("newbie", "newbie", None)?;

        let encrypted_req = auth_db
            .app_context
            .blueprint
            .extensions
            .auth
            .encrypt_aes(serde_json::to_string(&auth_req)?)?;
        let result = auth_db
            .handle_request(bytes::Bytes::from(encrypted_req))
            .await;

        assert!(result.success.is_some());
        let succ = result.success.unwrap();
        assert_eq!(succ.name, "newbie");
        Ok(())
    }

    #[tokio::test]
    async fn test_handle_req_signup() -> anyhow::Result<()> {
        let mut auth_db = get_db().await?;
        let admin = User {
            username: "admin".to_string(),
            name: "admin".to_string(),
            password: hash_256("admin"),
            authority: Authority::Admin,
            batch: None,
        };
        auth_db.users.insert(admin);
        let signup = SignUpDet {
            name: "newbie".to_string(),
            authority: 2,                        // is student
            admin_username: "admin".to_string(), // siged by: admin
            admin_password: "admin".to_string(),
            batch: Some("22BCS".to_string()),
        };

        let auth_req = AuthRequest::new("newbie", "newbie", Some(signup))?;

        let encrypted_req = auth_db
            .app_context
            .blueprint
            .extensions
            .auth
            .encrypt_aes(serde_json::to_string(&auth_req)?)?;
        let result = auth_db
            .handle_request(bytes::Bytes::from(encrypted_req))
            .await;

        assert!(result.success.is_some());
        let succ = result.success.unwrap();
        assert_eq!(succ.name, "newbie");
        Ok(())
    }
}
