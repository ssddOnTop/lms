use crate::config::batch_info::BatchInfo;
use crate::config::course_info::CourseInfo;
use crate::config::hash_algo::Algorithm;
use crate::is_default;
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use totp_rs::{Secret, TOTP};

// TODO: ADD DOCS!!

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub server: Server,
    pub auth: AuthInfo,
    pub courses: BTreeMap<String, CourseInfo>,
    pub batches: Vec<BatchInfo>,
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
#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    #[serde(default, skip_serializing_if = "is_default")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub workers: Option<usize>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub request_timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub timeout_key: Option<String>,
}

impl Server {
    pub fn get_workers(&self) -> usize {
        self.workers.unwrap_or(num_cpus::get())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthInfo {
    pub auth_db_path: String,
    pub totp: TotpSettings,
    pub aes_key: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TotpSettings {
    pub totp_secret: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub algo: Option<Algorithm>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub digits: Option<usize>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub period: Option<u64>, // step
}

impl TotpSettings {
    pub fn into_totp(self) -> Result<TOTP> {
        Ok(TOTP::new(
            self.algo.unwrap_or_default().into_totp(),
            self.digits.unwrap_or(6),
            1,
            self.period.unwrap_or(30),
            Secret::Raw(self.totp_secret.as_bytes().to_vec()).to_bytes()?,
        )?)
    }
}
