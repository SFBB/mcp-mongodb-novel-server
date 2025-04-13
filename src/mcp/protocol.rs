use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// MCP Protocol structs for requests and responses
#[derive(Debug, Serialize, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: MCPParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPParams {
    pub query: String,
    #[serde(default)]
    pub context: Vec<MCPContext>,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPContext {
    pub name: String,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub result: Option<MCPResult>,
    pub error: Option<MCPError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPResult {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}