use std::collections::HashMap;
use serde_json::Value;
use rmcp::model::CallToolResult;
use serde_json::to_string;

use crate::mcp::protocol::{MCPParams, MCPResult};

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