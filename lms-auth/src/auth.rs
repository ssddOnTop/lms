use crate::is_default;
use anyhow::{anyhow, Context, Result};
use libaes::AES_256_KEY_LEN;
use serde::{Deserialize, Serialize};
use totp_rs::TOTP;
use url::Url;

use crate::local_crypto::{decrypt_aes, encrypt_aes, gen_totp, hash_256};

#[derive(Debug, Clone)]
pub struct AuthProvider {
    auth_db_url: Url,
    totp: TOTP,
    aes_key: Vec<u8>,
}

#[derive(Serialize)]
struct AuthRequest {
    username: String,
    password: String,
    signature: String,
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
    message: String,
}

impl AuthRequest {
    fn new<T: AsRef<str>>(username: T, password: T, provider: &AuthProvider) -> Result<Self> {
        let password = hash_256(password);
        let totp = gen_totp(&provider.totp)?;
        let extra_hash = hash_256(format!("{}ssdd{}{}", totp, username.as_ref(), password));

        Ok(Self {
            username: username.as_ref().to_string(),
            password,
            signature: extra_hash,
        })
    }
    fn into_encrypted_request(self, aes_key: &[u8]) -> Result<String> {
        let request = encrypt_aes(aes_key, &serde_json::to_string(&self)?)
            .map_err(|_| anyhow!("Unable to encrypt request"))?;
        Ok(request)
    }
}

impl AuthResult {
    pub fn try_from_encrypted_response(aes_key: &[u8], response: &str) -> Result<Self> {
        let response =
            decrypt_aes(aes_key, response).map_err(|_| anyhow!("Unable to decrypt response"))?;
        let result = serde_json::from_str::<AuthResult>(&response)?;
        Ok(result)
    }
}

impl AuthProvider {
    pub fn init(auth_db_url: String, totp: TOTP, aes_key: String) -> Result<AuthProvider> {
        assert_eq!(aes_key.len(), AES_256_KEY_LEN, "AES key must be 16 bytes");

        let provider = Self {
            auth_db_url: Url::parse(&auth_db_url)?,
            totp,
            aes_key: aes_key.into_bytes(),
        };
        Ok(provider)
    }

    pub async fn authenticate(&self, username: &str, password: &str) -> Result<AuthSucc> {
        let request = AuthRequest::new(username, password, self)?;
        let request = request.into_encrypted_request(&self.aes_key)?;

        let response = reqwest::Client::new()
            .post(self.auth_db_url.clone())
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

        let req = AuthRequest::new("user", "pass", &provider)?;
        let req = req.into_encrypted_request(&provider.aes_key)?;
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
}
