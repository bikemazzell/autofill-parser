use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// Represents the data parsed from a single line initially.
// Using a HashMap to store key-value pairs for flexibility.
pub type RawRecord = HashMap<String, String>;

// Represents the final merged user record to be serialized to JSON.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UserOutput {
    pub identifier: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub emails: Vec<String>,
    #[serde(flatten)] // Flattens other key-value pairs into the main JSON object
    pub other_fields: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub memory_usage_percent: usize,
    pub temp_directory: String,
} 