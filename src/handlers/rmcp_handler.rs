// src/handlers/rmcp_handler.rs
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    body,
};
use std::sync::Arc;
use std::collections::HashMap;
use rmcp::{
    model::CallToolResult, Error, ServiceExt
};
use crate::mcp::server::MCPDatabaseServer;
use crate::services::DatabaseService;
use crate::mcp::{MCPRequest, MCPResponse, MCPResult, MCPError, MCPParams};
use crate::mcp::conversion::call_tool_result_to_mcp_result;
use serde_json::{json, Value, to_string};

impl From<CallToolResult> for MCPResult {
    fn from(result: CallToolResult) -> Self {
        let content = to_string(&result.content).unwrap_or_else(|_| "".to_string());

        MCPResult {
            content: content,
            metadata: Some(HashMap::from([
                ("token_count".to_string(), json!(-1)),
                ("metadata".to_string(), json!({})),
            ])),
        }
    }
}

// Handler for the HTTP endpoint for MCP requests
pub async fn rmcp_http_handler<T: DatabaseService + Clone + Send + Sync + 'static>(
    State(state): State<Arc<ServerState<T>>>,
    body: axum::body::Body,
) -> Response {
    // Create a server instance with our database service
    let mcp_server = MCPDatabaseServer::new(state.db_service.clone());
    
    // Convert HTTP request to JSON-RPC message
    // This is a simplified approach - for production, proper error handling should be added
    let bytes = match body::to_bytes(body, 1024 * 1024).await { // 1MB limit
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {}", e),
            )
                .into_response();
        }
    };
    
    let request_str = match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid UTF-8 in request body: {}", e),
            )
                .into_response();
        }
    };
    
    // Parse the JSON-RPC request
    let response = match serde_json::from_str::<MCPRequest>(&request_str) {
        Ok(request) => {
            // Validate JSON-RPC version
            if request.jsonrpc != "2.0" {
                return format_error_response(
                    request.id,
                    -32600,
                    "Invalid JSON-RPC version. Expected 2.0".to_string(),
                    None,
                )
                .into_response();
            }

            // Process based on the method
            let result = match request.method.as_str() {
                "query" => handle_query(&mcp_server, &request.params).await,
                "get_chapter_content" => handle_chapter_content(&mcp_server, &request.params).await,
                "get_character_details" => handle_character_details(&mcp_server, &request.params).await,
                "query_qa_regex" => handle_qa_regex(&mcp_server, &request.params).await,
                "query_chapter_regex" => handle_chapter_regex(&mcp_server, &request.params).await,
                "query_character_regex" => handle_character_regex(&mcp_server, &request.params).await,
                "update_chapter_summary" => handle_chapter_summary_update(&mcp_server, &request.params).await,
                // MCP standard methods
                "mcp.capabilities" => handle_capabilities().await,
                "mcp.prompts" => handle_prompts().await,
                "initialize" => handle_initialize(&request.params).await,
                "notifications/initialized" => {
                    tracing::info!("Received notifications/initialized");
                    // Per JSON-RPC 2.0, notifications do not expect a response (no id field)
                    if request.id.is_none() {
                        return format_notification_received().into_response();
                    }
                    // If id is present (should not happen), return a no-op result for compatibility
                    Ok(MCPResult {
                        content: "".to_string(),
                        metadata: None,
                    })
                },
                _ => Err(MCPError {
                    code: -32601, // Method not found
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            };

            match result {
                Ok(result) => {
                    // Create a successful JSON-RPC response
                    MCPResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(result),
                        error: None,
                    }
                },
                Err(e) => {
                    // Create an error JSON-RPC response
                    MCPResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(e),
                    }
                }
            }
        },
        Err(e) => {
            return format_error_response(
                None,
                -32700, // Parse error
                format!("Invalid JSON in request body: {}", e),
                None,
            )
            .into_response();
        }
    };
    
    // Return the JSON-RPC response
    let json_response = match serde_json::to_string(&response) {
        Ok(json) => json,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize response: {}", e),
            )
                .into_response();
        }
    };
    
    (StatusCode::OK, json_response).into_response()
}

// Handle database query requests
async fn handle_query<T: Clone + Send + Sync + 'static + DatabaseService>(
    mcp_server: &MCPDatabaseServer<T>,
    params: &MCPParams,
) -> Result<MCPResult, MCPError> {
    if let Some(query) = &params.query {
        mcp_server.handle_query(query)
            .await
            .map(|result| call_tool_result_to_mcp_result(result))
            .map_err(|e| convert_error(e))
    } else {
        Err(MCPError {
            code: -32602, // Invalid params
            message: "Missing query parameter".to_string(),
            data: None,
        })
    }
}

// Handle chapter content requests
async fn handle_chapter_content<T: Clone + Send + Sync + 'static + crate::services::db_service::DatabaseService>(
    mcp_server: &MCPDatabaseServer<T>,
    params: &MCPParams,
) -> Result<MCPResult, MCPError> {
    // Extract chapter_id from options
    let chapter_id = params.options.get("chapter_id")
        .and_then(|v| v.as_str())
        .ok_or(MCPError {
            code: -32602,
            message: "Missing chapter_id parameter".to_string(),
            data: None,
        })?;
    
    mcp_server.handle_chapter_content(chapter_id)
        .await
        .map(|result| call_tool_result_to_mcp_result(result))
        .map_err(|e| convert_error(e))
}

// Handle specific character details requests
async fn handle_character_details<T: Clone + Send + Sync + 'static + crate::services::db_service::DatabaseService>(
    mcp_server: &MCPDatabaseServer<T>,
    params: &MCPParams,
) -> Result<MCPResult, MCPError> {
    // Extract character_id from options
    let character_id = params.options.get("character_id")
        .and_then(|v| v.as_str())
        .ok_or(MCPError {
            code: -32602,
            message: "Missing character_id parameter".to_string(),
            data: None,
        })?;
    
    mcp_server.handle_character_details(character_id)
        .await
        .map(|result| call_tool_result_to_mcp_result(result))
        .map_err(|e| convert_error(e))
}

// Handle regex-based Q&A queries
async fn handle_qa_regex<T: Clone + Send + Sync + 'static + crate::services::db_service::DatabaseService>(
    mcp_server: &MCPDatabaseServer<T>,
    params: &MCPParams,
) -> Result<MCPResult, MCPError> {
    // Extract regex_pattern from options
    let regex_pattern = params.options.get("regex_pattern")
        .and_then(|v| v.as_str())
        .ok_or(MCPError {
            code: -32602,
            message: "Missing regex_pattern parameter".to_string(),
            data: None,
        })?;
    
    mcp_server.handle_qa_regex(regex_pattern)
        .await
        .map(|result| call_tool_result_to_mcp_result(result))
        .map_err(|e| convert_error(e))
}

// Handle regex-based chapter queries
async fn handle_chapter_regex<T: Clone + Send + Sync + 'static + crate::services::db_service::DatabaseService>(
    mcp_server: &MCPDatabaseServer<T>,
    params: &MCPParams,
) -> Result<MCPResult, MCPError> {
    // Extract regex_pattern from options
    let regex_pattern = params.options.get("regex_pattern")
        .and_then(|v| v.as_str())
        .ok_or(MCPError {
            code: -32602,
            message: "Missing regex_pattern parameter".to_string(),
            data: None,
        })?;
    
    mcp_server.handle_chapter_regex(regex_pattern)
        .await
        .map(Into::into)
        .map_err(|e| convert_error(e))
}

// Handle regex-based character queries
async fn handle_character_regex<T: Clone + Send + Sync + 'static + crate::services::db_service::DatabaseService>(
    mcp_server: &MCPDatabaseServer<T>,
    params: &MCPParams,
) -> Result<MCPResult, MCPError> {
    // Extract regex_pattern from options
    let regex_pattern = params.options.get("regex_pattern")
        .and_then(|v| v.as_str())
        .ok_or(MCPError {
            code: -32602,
            message: "Missing regex_pattern parameter".to_string(),
            data: None,
        })?;
    
    mcp_server.handle_character_regex(regex_pattern)
        .await
        .map(Into::into)
        .map_err(|e| convert_error(e))
}

// Handle chapter summary updates
async fn handle_chapter_summary_update<T: Clone + Send + Sync + 'static + crate::services::db_service::DatabaseService>(
    mcp_server: &MCPDatabaseServer<T>,
    params: &MCPParams,
) -> Result<MCPResult, MCPError> {
    // Extract chapter_id and summary from options
    let chapter_id = params.options.get("chapter_id")
        .and_then(|v| v.as_str())
        .ok_or(MCPError {
            code: -32602,
            message: "Missing chapter_id parameter".to_string(),
            data: None,
        })?;
    
    let summary = params.options.get("summary")
        .and_then(|v| v.as_str())
        .ok_or(MCPError {
            code: -32602,
            message: "Missing summary parameter".to_string(),
            data: None,
        })?;
    
    mcp_server.handle_chapter_summary_update(chapter_id, summary)
        .await
        .map(Into::into)
        .map_err(|e| convert_error(e))
}

// Handle MCP capabilities method
async fn handle_capabilities() -> Result<MCPResult, MCPError> {
    let capabilities = json!({
        "name": "MCP MongoDB Server",
        "version": "1.0.0",
        "methods": [
            "query",
            "get_chapter_content",
            "get_character_details",
            "query_qa_regex",
            "query_chapter_regex",
            "query_character_regex",
            "update_chapter_summary",
            "mcp.capabilities",
            "mcp.prompts"
        ],
        "standardMethods": ["mcp.capabilities", "mcp.prompts", "initialize"],
        "description": "An optimized MCP server for MongoDB databases, designed for small context windows (3k tokens)"
    });
    
    Ok(MCPResult {
        content: "".to_string(), // No content needed for capabilities
        metadata: Some(HashMap::from([
            ("capabilities".to_string(), capabilities)
        ])),
    })
}

// Handle MCP prompts method
async fn handle_prompts() -> Result<MCPResult, MCPError> {
    let prompts = vec![
        "Search for chapters about dragons",
        "Find characters with supernatural abilities",
        "Get information about the magic system",
        "Retrieve Q&A entries about world history",
        "Find all chapters where a specific character appears"
    ];
    
    Ok(MCPResult {
        content: prompts.join("\n"),
        metadata: Some(HashMap::from([
            ("token_count".to_string(), json!(prompts.iter().map(|p| p.len()).sum::<usize>() / 4))
        ])),
    })
}

// Handle initialize method
async fn handle_initialize(params: &MCPParams) -> Result<MCPResult, MCPError> {
    // MCP initialize is primarily for version negotiation
    // Check client capabilities and version from options
    let client_version = params.options.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0");
    
    let client_name = params.options.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Client");
    
    tracing::info!("Client initialization: {} ({})", client_name, client_version);
    
    // Return server capabilities
    let capabilities = json!({
        "name": "MCP MongoDB Server",
        "version": "1.0.0",
        "optimized_for_small_context": true,
        "context_size": 3000, // 3k tokens
        "protocol": "MCP 1.0"
    });
    
    Ok(MCPResult {
        content: "".to_string(),
        metadata: Some(HashMap::from([
            ("server_info".to_string(), capabilities)
        ])),
    })
}

// Helper to format error responses
fn format_error_response(
    id: Option<Value>, 
    code: i32, 
    message: String, 
    data: Option<Value>
) -> Response {
    let error_response = MCPResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(MCPError {
            code,
            message,
            data,
        }),
    };
    
    match serde_json::to_string(&error_response) {
        Ok(json) => (StatusCode::OK, json).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize error response: {}", e),
        ).into_response(),
    }
}

// Helper for handling notification messages
fn format_notification_received() -> Response {
    (StatusCode::NO_CONTENT, "").into_response()
}

// Convert rmcp::Error to MCPError
fn convert_error(error: Error) -> MCPError {
    MCPError {
        code: error.code.0,
        message: error.message.to_string(),
        data: error.data,
    }
}

// For stdio interface replacement
pub async fn run_stdio_mcp_server<T: DatabaseService + Clone + Send + Sync + 'static>(
    db_service: T,
) -> anyhow::Result<()> {
    use tokio::io::{stdin, stdout};
    
    // Create a server instance with our database service
    let mcp_server = MCPDatabaseServer::new(db_service);
    
    // Use stdin/stdout for transport
    let transport = (stdin(), stdout());
    
    // Start serving
    let server = mcp_server.serve(transport).await?;
    
    // Wait for the server to exit
    server.waiting().await?;
    
    Ok(())
}

pub struct ServerState<T: DatabaseService> {
    pub db_service: T,
}