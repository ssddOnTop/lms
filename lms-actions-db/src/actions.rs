use std::collections::HashMap;
use http_body_util::Full;
use serde::{Deserialize, Serialize};
use lms_core::is_default;
use anyhow::{anyhow, Result};
use lms_auth::auth::AuthProvider;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionsResult {
    #[serde(default)]
    pub status: u16,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionsRequest {
    pub token: String,
    pub group_id: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub read: Option<ActionsRead>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub write: Option<ActionsWrite>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionsRead {
    pub content_id: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub file_name: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionsWrite {
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub files: Option<FileWrite>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub end_time: Option<u128>,
    pub reference: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileWrite {
    pub file_name: String,
    pub content: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ActionsActivity {
    pub actions: HashMap<String, ActionsContent>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ActionsContent {
    pub is_notif: bool,
    pub content_id: String,
}

impl ActionsActivity {
    pub fn insert(&mut self, group_id: &str, content_id: &str, is_notif: bool) {
        // TODO insert it in file-db as well
        self.actions.insert(
            group_id.to_string(),
            ActionsContent {
                is_notif,
                content_id: content_id.to_string(),
            },
        );
    }
    pub fn get(&self, group_id: &str) -> Option<&ActionsContent> {
        self.actions.get(group_id)
    }
    /*
     pub fn get_file(&self, group_id: &str) -> Option<&ActionsContent> {
           let content_id = self.get(group_id);
           todo!()
       }
    */
}

impl ActionsResult {
    pub fn into_hyper_response(self) -> Result<hyper::Response<Full<bytes::Bytes>>> {
        let body = serde_json::to_string(&self)?;
        let response = hyper::Response::builder()
            .status(self.status)
            .header("Content-Type", "application/json")
            .body(Full::new(bytes::Bytes::from(body)))?;
        Ok(response)
    }
}

impl ActionsRequest {
    pub fn try_from_encrypted<T: AsRef<[u8]>>(
        req: T,
        auth_provider: &AuthProvider,
    ) -> Result<Self> {
        let req = auth_provider
            .decrypt_aes(req)
            .map_err(|_| anyhow!("Unable to decrypt request"))?;
        let req =
            serde_json::from_str::<Self>(&req).map_err(|_| anyhow!("Unable to parse request"))?;
        Ok(req)
    }
}