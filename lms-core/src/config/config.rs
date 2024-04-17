use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    pub port: u16,
    pub auth: AuthInfo,
}

impl Config {
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
    pub fn to_json(&self, pretty: bool) -> Result<String> {
        if pretty {
            Ok(serde_json::to_string_pretty(self)?)
        } else {
            Ok(serde_json::to_string(self)?)
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AuthInfo {
    pub auth_url: String,
    pub totp_key: String,
    pub aes_key: String,
}
