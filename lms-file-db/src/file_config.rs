use serde::{Deserialize, Deserializer, Serialize, Serializer};
use anyhow::Result;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde::ser::SerializeStruct;
use lms_core::authdb::auth_actors::Authority;
use lms_core::is_default;

#[derive(Clone)]
pub struct FileHolder {
    pub name: String,
    pub content: Vec<u8>,
}



pub struct InsertionInfo {
    pub title: String,
    pub description: String,
    pub timestamp: u128,
    pub end_time: Option<u128>,
    pub authority: Authority,
}

impl Serialize for FileHolder {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer {
        let mut state = serializer.serialize_struct("FileHolder", 2)?;
        state.serialize_field("name", &self.name)?;
        let b64 = BASE64_STANDARD.encode(&self.content);
        state.serialize_field("content", &b64)?;
        state.end()
    }
}

struct FileHolderVisitor;

impl<'de> serde::de::Visitor<'de> for FileHolderVisitor {
    type Value = FileHolder;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct FileHolder")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error> where A: serde::de::MapAccess<'de> {
        let mut name = None;
        let mut content = None;
        while let Some(key) = map.next_key()? {
            match key {
                "name" => {
                    if name.is_some() {
                        return Err(serde::de::Error::duplicate_field("name"));
                    }
                    name = Some(map.next_value()?);
                }
                "content" => {
                    if content.is_some() {
                        return Err(serde::de::Error::duplicate_field("content"));
                    }
                    let b64: String = map.next_value()?;
                    content = Some(BASE64_STANDARD.decode(b64.as_bytes()).map_err(serde::de::Error::custom)?);
                }
                _ => {
                    return Err(serde::de::Error::unknown_field(key, &["name", "content"]));
                }
            }
        }
        let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
        let content = content.ok_or_else(|| serde::de::Error::missing_field("content"))?;
        Ok(FileHolder { name, content })
    }
}

impl<'de> Deserialize<'de> for FileHolder {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_struct("FileHolder", &["name", "content"], FileHolderVisitor)
    }
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub title: String,
    pub description: String,
    pub timestamp: u128,
    #[serde(default, skip_serializing_if = "is_default")]
    pub end_time: Option<u128>,
    pub authority: Authority,
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
                authority: insertion_info.authority,
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

    pub fn combine_info(insertion_info: InsertionInfo, files: &Vec<FileHolder>) -> Self {
        Self {
            files: files.iter().map(|file| file.name.clone()).collect(),
            metadata: Metadata {
                title: insertion_info.title,
                description: insertion_info.description,
                timestamp: insertion_info.timestamp,
                end_time: insertion_info.end_time,
                authority: insertion_info.authority,
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