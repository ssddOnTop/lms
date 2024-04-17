use crate::config::hash_algo::Algorithm;
use crate::is_default;
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: ADD DOCS!!

#[derive(Default, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Config {
    #[serde(default, skip_serializing_if = "is_default")]
    pub server: Server,
    #[serde(default, skip_serializing_if = "is_default")]
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
#[derive(Default, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Server {
    #[serde(default, skip_serializing_if = "is_default")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub workers: Option<usize>,
}

impl Server {
    pub fn get_workers(&self) -> usize {
        self.workers.unwrap_or(num_cpus::get())
    }
}

#[derive(Default, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AuthInfo {
    #[serde(default, skip_serializing_if = "is_default")]
    pub auth_url: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub totp: TotpSettings,
    #[serde(default, skip_serializing_if = "is_default")]
    pub aes_key: String,
}

#[derive(Default, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct TotpSettings {
    #[serde(default, skip_serializing_if = "is_default")]
    pub totp_secret: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub algo: Option<Algorithm>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub digits: Option<usize>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub period: Option<u64>, // step
}
