use crate::file_db::file_config::{FileHolder, InsertionInfo, Metadata};
use crate::file_db::request_handler::FileRequestHandler;
use crate::is_default;
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use http_body_util::Full;

use serde::{Deserialize, Serialize};

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
    pub files: Option<Vec<FileWrite>>,
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
    pub actions: DashMap<String, Vec<ActionsContent>>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ActionsContent {
    pub is_notif: bool,
    pub content_id: String,
}

impl ActionsActivity {
    pub async fn insert(
        &self,
        group_id: String,
        info: InsertionInfo,
        files: Vec<FileHolder>,
        file_request_handler: &FileRequestHandler,
        is_notif: bool,
    ) -> Result<String> {
        let content_id = file_request_handler.insert(info, files).await?;
        let new_action = ActionsContent {
            is_notif,
            content_id: content_id.clone(),
        };

        if let Some(mut actions) = self.get_actions(&group_id) {
            actions.push(new_action);
            self.actions.insert(group_id.to_string(), actions);
        } else {
            self.actions.insert(group_id.to_string(), vec![new_action]);
        }
        Ok(content_id)
    }
    pub fn get_actions(&self, group_id: &str) -> Option<Vec<ActionsContent>> {
        let val = self.actions.get(group_id)?;
        Some(val.value().clone())
    }
    pub async fn get_config(
        &self,
        content_id: &str,
        file_request_handler: &FileRequestHandler,
    ) -> Result<Metadata> {
        file_request_handler.get_metadata(content_id).await
    }
    pub async fn get_file_content(
        &self,
        content_id: &str,
        file_name: &str,
        file_request_handler: &FileRequestHandler,
    ) -> Result<FileHolder> {
        file_request_handler.get(content_id, file_name).await
    }
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
    pub fn into_serrequet(self) -> Result<String> {
        let request =
            serde_json::to_string(&self).map_err(|_| anyhow!("Unable to encode request"))?;
        Ok(request)
    }
    pub fn try_from_bytes<T: AsRef<[u8]>>(req: T) -> Result<Self> {
        let req = serde_json::from_slice::<Self>(req.as_ref())
            .map_err(|_| anyhow!("Unable to parse request"))?;
        Ok(req)
    }
}
