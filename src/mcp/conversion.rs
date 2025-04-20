use std::collections::HashMap;
use std::borrow::Cow;
use serde_json::Value;
use rmcp::model::{CallToolResult, Content, Annotated, PromptMessageContent};
use serde_json::to_string;
use serde_json::json;
use std::error::Error;
use uuid::Uuid;

use crate::mcp::protocol::{MCPParams, MCPResult};

// Define basic structures for MPC protocol
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MpcRequest {
    pub id: String,
    pub q: String,
    pub ctx: RequestContext,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MpcResponse {
    pub id: String,
    pub status: i32,
    pub content: Value,
    pub ctx: RequestContext,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestContext {
    pub request_id: String,
    pub token_count: i32,
    pub max_tokens: i32,
    pub remaining_tokens: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MpcError {
    pub code: i32,
    pub message: String,
}

impl MpcError {
    pub fn new(code: i32, message: String) -> Self {
        Self { code, message }
    }
}

// Extension trait to add from_raw functionality to Content
pub trait ContentExt {
    fn from_raw<T: Into<String>>(text: T) -> Self;
}

impl ContentExt for Content {
    fn from_raw<T: Into<String>>(text: T) -> Self {
        Content {
            raw: rmcp::model::RawContent::text(text),
            annotations: None,
        }
    }
}

// Helper function to create PromptMessageContent from a string
pub fn create_message_content(text: &str) -> PromptMessageContent {
    PromptMessageContent::Text { 
        text: text.to_string()
    }
}

// Convert our internal MCPParams to RMCP-compatible format
pub fn mcp_params_to_rmcp_params(params: MCPParams) -> HashMap<String, Value> {
    let mut rmcp_params = HashMap::new();
    
    if let Some(query) = params.query {
        rmcp_params.insert("query".to_string(), Value::String(query));
    }
    
    if !params.context.is_empty() {
        let context: Vec<Value> = params.context.into_iter().map(|ctx| {
            let mut context_obj = serde_json::Map::new();
            context_obj.insert("name".to_string(), Value::String(ctx.name));
            if let Some(content) = ctx.content {
                context_obj.insert("content".to_string(), Value::String(content));
            }
            Value::Object(context_obj)
        }).collect();
        
        rmcp_params.insert("context".to_string(), Value::Array(context));
    }
    
    if !params.options.is_empty() {
        rmcp_params.insert("options".to_string(), Value::Object(
            params.options.into_iter()
                .map(|(k, v)| (k, v))
                .collect()
        ));
    }
    
    rmcp_params
}

// Convert CallToolResult to MCPResult
pub fn call_tool_result_to_mcp_result(result: CallToolResult) -> MCPResult {
    let mut metadata = HashMap::new();
    
    // Use the return_value or content as the main content
    let content = to_string(&result.content).unwrap_or_else(|_| "".to_string());
    
    MCPResult {
        content,
        metadata: if metadata.is_empty() { None } else { Some(metadata) },
    }
}