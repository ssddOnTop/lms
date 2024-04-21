#![allow(unused)]

use crate::authdb::auth_actors::Authority;
use crate::is_default;
use anyhow::Result;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHolder {
    pub name: String,
    pub content: String,
}

pub struct InsertionInfo {
    pub title: String,
    pub description: String,
    pub timestamp: u128,
    pub end_time: Option<u128>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct RemoteFileConfig {
    #[serde(default, skip_serializing_if = "is_default")]
    pub files: Vec<FileHolder>, // file names and content
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize)]
pub struct LocalFileConfig {
    #[serde(default, skip_serializing_if = "is_default")]
    pub files: Vec<String>, // path of files in the same dir
    pub metadata: Metadata,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct Metadata {
    pub title: String,
    pub description: String,
    pub timestamp: u128,
    #[serde(default, skip_serializing_if = "is_default")]
    pub end_time: Option<u128>,
}

impl RemoteFileConfig {
    pub fn combine_info(insertion_info: InsertionInfo, files: Vec<FileHolder>) -> Self {
        Self {
            files,
            metadata: Metadata {
                title: insertion_info.title,
                description: insertion_info.description,
                timestamp: insertion_info.timestamp,
                end_time: insertion_info.end_time,
            },
        }
    }
    pub fn serialize(self) -> Result<String> {
        Ok(serde_json::to_string(&self)?)
    }
    pub fn deserialize(data: &str) -> Result<Self> {
        Ok(serde_json::from_str(data)?)
    }
}

impl LocalFileConfig {
    pub fn combine_info(insertion_info: InsertionInfo, files: &[FileHolder]) -> Self {
        Self {
            files: files.iter().map(|file| file.name.clone()).collect(),
            metadata: Metadata {
                title: insertion_info.title,
                description: insertion_info.description,
                timestamp: insertion_info.timestamp,
                end_time: insertion_info.end_time,
            },
        }
    }

    pub fn serialize(self) -> Result<String> {
        Ok(serde_json::to_string(&self)?)
    }
    pub fn deserialize(data: &str) -> Result<Self> {
        Ok(serde_json::from_str(data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_file_holder() {
        let content = "Hello, world!";
        let file_holder = FileHolder {
            name: "example.txt".to_string(),
            content: content.to_string(),
        };
        let serialized = serde_json::to_string(&file_holder).unwrap();
        insta::assert_snapshot!(serialized);
    }

    #[test]
    fn test_serialize_remote_file_config() {
        let files = vec![FileHolder {
            name: "doc.txt".to_string(),
            content: "Sample content".to_string(),
        }];
        let metadata = Metadata {
            title: "Data Collection".to_string(),
            description: "Project files".to_string(),
            timestamp: 1625247600000,
            end_time: None,
        };
        let config = RemoteFileConfig { files, metadata };
        let serialized = serde_json::to_string(&config).unwrap();
        insta::assert_snapshot!(serialized);
    }

    #[test]
    fn test_deserialize_file_holder() {
        let json = r#"{"name":"example.txt","content":"SGVsbG8sIHdvcmxkIQ=="}"#;
        let file_holder: FileHolder = serde_json::from_str(json).unwrap();
        assert_eq!(file_holder.name, "example.txt");
        assert_eq!(file_holder.content, "SGVsbG8sIHdvcmxkIQ==");
    }

    #[test]
    fn test_deserialize_file_holder_with_missing_fields() {
        let json = r#"{"name":"example.txt"}"#;
        let result: Result<FileHolder, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_file_holder_with_duplicate_fields() {
        let json = r#"{"name":"example.txt","name":"test.txt","content":"SGVsbG8sIHdvcmxkIQ=="}"#;
        let result: Result<FileHolder, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
    #[test]
    fn test_round_trip_file_holder() {
        let original = FileHolder {
            name: "roundtrip.txt".to_string(),
            content: "Round trip test".to_string(),
        };
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: FileHolder = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }
}
