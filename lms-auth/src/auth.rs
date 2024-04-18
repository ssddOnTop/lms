use crate::is_default;
use anyhow::{anyhow, Context, Result};
use http_body_util::Full;
use libaes::AES_256_KEY_LEN;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use totp_rs::TOTP;

use crate::local_crypto::{decrypt_aes, encrypt_aes, gen_totp, hash_128, hash_256};

#[derive(Debug, Clone)]
pub struct AuthProvider {
    auth_db_path: String,
    totp: TOTP,
    aes_key: Vec<u8>,
}

pub enum RequestType {
    Login,
    Signup,
}

impl Display for RequestType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestType::Login => f.write_str("login"),
            RequestType::Signup => f.write_str("signup"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
    pub signature: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub signup_details: Option<SignUpDet>,
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct SignUpDet {
    pub name: String,
    pub authority: u8,
    pub admin_username: String,
    pub admin_password: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthResult {
    #[serde(default, skip_serializing_if = "is_default")]
    pub error: Option<AuthError>,

    #[serde(default, skip_serializing_if = "is_default")]
    pub success: Option<AuthSucc>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthSucc {
    #[serde(default, skip_serializing_if = "is_default")]
    pub name: String,
    pub token: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthError {
    #[serde(default, skip_serializing_if = "is_default")]
    pub message: String,
}

impl AuthRequest {
    fn new<T: AsRef<str>>(
        username: T,
        password: T,
        provider: &AuthProvider,
        sign_up_det: Option<SignUpDet>,
    ) -> Result<Self> {
        let password = hash_256(password);
        let extra_hash = provider.gen_sig(username.as_ref(), &password)?;

        let mut signup_details = None;

        if let Some(mut signup_det) = sign_up_det {
            signup_det.admin_password = hash_256(signup_det.admin_password);
            signup_details = Some(signup_det);
        }

        Ok(Self {
            username: username.as_ref().to_string(),
            password,
            signature: extra_hash,
            signup_details,
        })
    }
    fn into_encrypted_request(self, auth_provider: &AuthProvider) -> Result<String> {
        let request = auth_provider
            .encrypt_aes(serde_json::to_string(&self)?)
            .map_err(|_| anyhow!("Unable to encrypt request"))?;
        Ok(request)
    }
    pub fn try_from_encrypted<T: AsRef<[u8]>>(
        req: T,
        auth_provider: &AuthProvider,
    ) -> Result<Self> {
        let req = auth_provider.decrypt_aes(req)?;
        println!("hx");
        let req = serde_json::from_str::<Self>(&req)?;
        Ok(req)
    }
    pub fn verify_sig(&self, auth_provider: &AuthProvider) -> bool {
        let sig = auth_provider.gen_sig(&self.username, &self.password);
        println!("L: {:?}", sig);
        println!("R: {}", self.signature);
        sig.map(|v| v.eq(&self.signature)).unwrap_or(false)
    }
}

impl AuthResult {
    pub fn try_from_encrypted_response(aes_key: &[u8], response: &str) -> Result<Self> {
        let response =
            decrypt_aes(aes_key, response).map_err(|_| anyhow!("Unable to decrypt response"))?;
        let result = serde_json::from_str::<AuthResult>(&response)?;
        Ok(result)
    }
    pub fn into_hyper_response(self) -> Result<hyper::Response<Full<bytes::Bytes>>> {
        let body = serde_json::to_string(&self)?;
        let response = hyper::Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Full::new(bytes::Bytes::from(body)))?;
        Ok(response)
    }
}

impl AuthProvider {
    pub fn init(auth_db_path: String, totp: TOTP, aes_key: String) -> Result<AuthProvider> {
        let aes_key = hash_128(aes_key)[..32].to_string();
        assert_eq!(aes_key.len(), AES_256_KEY_LEN, "AES key must be 16 bytes");

        let provider = Self {
            auth_db_path,
            totp,
            aes_key: aes_key.into_bytes(),
        };
        Ok(provider)
    }

    pub async fn authenticate(&self, username: &str, password: &str) -> Result<AuthSucc> {
        let request = AuthRequest::new(username, password, self, None)?;
        let request = request.into_encrypted_request(self)?;

        let response = reqwest::Client::new()
            .post(self.auth_db_path.clone())
            .body(request)
            .send()
            .await?;

        let result =
            AuthResult::try_from_encrypted_response(&self.aes_key, &response.text().await?)?;

        if let Some(err) = result.error {
            return Err(anyhow::anyhow!(err.message));
        }

        result.success.context("Internal error: Empty response")
    }
    pub fn encrypt_aes<T: AsRef<str>>(&self, content: T) -> Result<String> {
        encrypt_aes(&self.aes_key, content.as_ref())
    }

    pub fn decrypt_aes<T: AsRef<[u8]>>(&self, content: T) -> Result<String> {
        decrypt_aes(&self.aes_key, content.as_ref())
    }
    pub fn gen_sig(&self, a: &str, b: &str) -> Result<String> {
        let totp = gen_totp(&self.totp)?;
        Ok(hash_256(format!("{}ssdd{}{}", totp, a, b)))
    }
    pub fn db_path(&self) -> &str {
        &self.auth_db_path
    }
    pub fn get_pw(&self) -> &[u8] {
        &self.aes_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use totp_rs::{Algorithm, Secret};

    #[test]
    fn test_deser_auth_resp() {
        let json = r#"{
            "success": {
                "token": "token"
            }
        }"#;

        let json_error = r#"{
            "error": {
                "message": "error message"
            }
        }"#;

        let result = serde_json::from_str::<AuthResult>(json);
        let result_err = serde_json::from_str::<AuthResult>(json_error);

        assert!(result.unwrap().success.is_some());
        assert!(result_err.unwrap().error.is_some());
    }

    fn start_mock_server() -> httpmock::MockServer {
        httpmock::MockServer::start()
    }

    #[tokio::test]
    async fn test_authenticate_success() -> Result<()> {
        let server = start_mock_server();

        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            Secret::Raw("JBSWY3DPEHPK3PXP".as_bytes().to_vec())
                .to_bytes()
                .unwrap(),
        )
        .unwrap();

        let aes_key = "12345678901234561234567890123456".to_string();
        let server_url = server.base_url();

        let provider = AuthProvider::init(server_url, totp, aes_key)?;

        let req = AuthRequest::new("user", "pass", &provider, None)?;
        let req = req.into_encrypted_request(&provider)?;
        let resp_json = json!({
            "success": {
                "name": "John Doe",
                "token": "abcdef123456"
            }
        });

        let resp = encrypt_aes(&provider.aes_key, &serde_json::to_string(&resp_json)?)?;

        let m = server.mock(|when, then| {
            when.method(httpmock::Method::POST).path("/").body(req);

            then.status(200).body(resp);
        });

        let result = provider.authenticate("user", "pass").await;

        m.assert();
        assert!(result.is_ok());

        let success = result.unwrap();
        assert_eq!(success.name, "John Doe");
        assert_eq!(success.token, "abcdef123456");

        Ok(())
    }

    /*    #[tokio::test]
    async fn test_req_gen() -> Result<()> { // TODO DROP THIS
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            Secret::Raw("base32encodedkey".as_bytes().to_vec()).to_bytes()?,
        )?;
        let auth = AuthProvider::init(
            "/home/ssdd/RustroverProjects/lms/auth".to_string(),
            totp,
            hash_256("32bytebase64encodedkey"),
        )?;

        let signup = SignUpDet {
            name: "fooo".to_string(),
            authority: 0,
            admin_username: "admin".to_string(),
            admin_password: "tmppw".to_string(),
        };

        let req = AuthRequest::new("user", "pass", &auth, Some(signup))?;
        let resp = reqwest::Client::new()
            .post("http://localhost:19194/auth")
            .body(req.into_encrypted_request(&auth)?)
            .send()
            .await?;
        println!("{:?}", resp.text().await);
        Ok(())
    }*/
}
