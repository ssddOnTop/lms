use crate::is_default;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CourseInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub description: Option<String>,
}
