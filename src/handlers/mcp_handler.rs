use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use std::collections::HashMap;

use crate::mcp::{MCPError, MCPParams, MCPRequest, MCPResponse, MCPResult};
use crate::models::SearchParams;
use crate::services::DatabaseService;
use crate::utils::QueryParser;

pub struct ServerState<T: DatabaseService> {
    pub db_service: T,
}

// Handler for MCP requests
pub async fn mcp_handler<T: DatabaseService>(
    State(state): State<Arc<ServerState<T>>>,
    Json(request): Json<MCPRequest>,
) -> Result<Json<MCPResponse>, MCPErrorResponse> {
    // Validate JSON-RPC request
    if request.jsonrpc != "2.0" {
        return Err(MCPErrorResponse {
            code: -32600,
            message: "Invalid JSON-RPC request".to_string(),
        });
    }

    // Process based on the method
    let result = match request.method.as_str() {
        "query" => handle_query(&state.db_service, request.params).await,
        "get_chapter_content" => handle_chapter_content(&state.db_service, request.params).await,
        "get_character_details" => handle_character_details(&state.db_service, request.params).await,
        "query_qa_regex" => handle_qa_regex_query(&state.db_service, request.params).await,
        "query_chapter_regex" => handle_chapter_regex_query(&state.db_service, request.params).await,
        "query_character_regex" => handle_character_regex_query(&state.db_service, request.params).await,
        "mcp.capabilities" => handle_capabilities().await,
        "mcp.prompts" => handle_prompts().await,
        // Conditionally expose write-access methods
        #[cfg(feature = "mcp_write_access")]
        "update_chapter_summary" => handle_update_chapter_summary(&state.db_service, request.params).await,
        #[cfg(not(feature = "mcp_write_access"))]
        "update_chapter_summary" => Err(MCPErrorResponse {
            code: -32601,
            message: format!("Method '{}' not found", request.method),
        }),
        _ => Err(MCPErrorResponse {
            code: -32601,
            message: format!("Method '{}' not found", request.method),
        }),
    }?;

    // Construct successful response
    Ok(Json(MCPResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id,
        result: Some(result),
        error: None,
    }))
}

// Handle database query requests
async fn handle_query<T: DatabaseService>(
    db_service: &T,
    params: MCPParams,
) -> Result<MCPResult, MCPErrorResponse> {
    // Parse the natural language query into structured params
    let query = params.query.as_ref().ok_or(MCPErrorResponse {
        code: -32602,
        message: "Missing 'query' field in params".to_string(),
    })?;
    let search_params = QueryParser::parse_natural_language_query(query);
    
    // Execute the appropriate search based on collection type
    let db_response = match search_params.collection.as_str() {
        "novels" => db_service.search_novels(&search_params).await,
        "chapters" => db_service.search_chapters(&search_params).await,
        "characters" => db_service.search_characters(&search_params).await,
        "qa" => db_service.search_qa(&search_params).await,
        _ => {
            return Err(MCPErrorResponse {
                code: -32602,
                message: format!("Unknown collection type: {}", search_params.collection),
            });
        }
    };

    // Handle database errors
    let db_result = match db_response {
        Ok(result) => result,
        Err(e) => {
            return Err(MCPErrorResponse {
                code: -32603,
                message: format!("Database error: {}", e),
            });
        }
    };

    // Format result for MCP protocol
    let content = format_content_for_llm(&db_result.data, &search_params);
    
    let metadata_map: HashMap<String, serde_json::Value> = [
        ("token_count".to_string(), serde_json::to_value(db_result.metadata.token_count).unwrap_or(serde_json::Value::Null)),
        ("query_time_ms".to_string(), serde_json::to_value(db_result.metadata.query_time_ms).unwrap_or(serde_json::Value::Null)),
        ("has_more".to_string(), serde_json::to_value(db_result.metadata.has_more).unwrap_or(serde_json::Value::Null)),
        ("next_page_token".to_string(), serde_json::to_value(db_result.metadata.next_page_token).unwrap_or(serde_json::Value::Null)),
    ].into_iter().collect();
    
    Ok(MCPResult {
        content,
        metadata: Some(metadata_map),
    })
}

// Handle specific chapter content requests
async fn handle_chapter_content<T: DatabaseService>(
    _db_service: &T,
    params: MCPParams,
) -> Result<MCPResult, MCPErrorResponse> {
    // This would query for specific chapter content
    // Implementation simplified for example purposes
    
    Ok(MCPResult {
        content: format!("Chapter content for query: {}", params.query.as_deref().unwrap_or("<none>")),
        metadata: None,
    })
}

// Handle specific character details requests
async fn handle_character_details<T: DatabaseService>(
    _db_service: &T,
    params: MCPParams,
) -> Result<MCPResult, MCPErrorResponse> {
    // This would query for detailed character information
    // Implementation simplified for example purposes
    
    Ok(MCPResult {
        content: format!("Character details for query: {}", params.query.as_deref().unwrap_or("<none>")),
        metadata: None,
    })
}

// Handle regex-based Q&A queries
async fn handle_qa_regex_query<T: DatabaseService>(
    db_service: &T,
    params: MCPParams,
) -> Result<MCPResult, MCPErrorResponse> {
    let regex_pattern = params.query.as_deref().unwrap_or("");
    let qa_entries = db_service.search_qa_by_regex(regex_pattern).await?;
    let content = format_qa(&qa_entries);

    Ok(MCPResult {
        content,
        metadata: None,
    })
}

// Handle regex-based chapter queries
async fn handle_chapter_regex_query<T: DatabaseService>(
    db_service: &T,
    params: MCPParams,
) -> Result<MCPResult, MCPErrorResponse> {
    let regex_pattern = params.query.as_deref().unwrap_or("");
    let chapters = db_service.search_chapters_by_regex(regex_pattern).await?;
    let content = format_chapters(&chapters);

    Ok(MCPResult {
        content,
        metadata: None,
    })
}

// Handle regex-based character queries
async fn handle_character_regex_query<T: DatabaseService>(
    db_service: &T,
    params: MCPParams,
) -> Result<MCPResult, MCPErrorResponse> {
    let regex_pattern = params.query.as_deref().unwrap_or("");
    let characters = db_service.search_characters_by_regex(regex_pattern).await?;
    let content = format_characters(&characters);

    Ok(MCPResult {
        content,
        metadata: None,
    })
}

// Handle the mcp.capabilities request
async fn handle_capabilities() -> Result<MCPResult, MCPErrorResponse> {
    let methods = vec![
        serde_json::json!({
            "method": "query_character",
            "description": "Retrieve detailed information about a character.",
            "parameters": { "character_id": "string" }
        }),
        serde_json::json!({
            "method": "query_novel",
            "description": "Retrieve metadata about a novel.",
            "parameters": { "novel_id": "string" }
        }),
        serde_json::json!({
            "method": "query_chapter",
            "description": "Retrieve information about a specific chapter by number, title, or ID.",
            "parameters": { "chapter_id": "string", "chapter_number": "integer", "chapter_title": "string" }
        }),
        serde_json::json!({
            "method": "query_qa_regex",
            "description": "Retrieve a list of Q&A entries matching a regex pattern.",
            "parameters": { "regex_pattern": "string" }
        }),
        serde_json::json!({
            "method": "query_chapter_regex",
            "description": "Retrieve a list of chapters matching a regex pattern.",
            "parameters": { "regex_pattern": "string" }
        }),
        serde_json::json!({
            "method": "query_character_regex",
            "description": "Retrieve a list of characters matching a regex pattern.",
            "parameters": { "regex_pattern": "string" }
        }),
    ];
    // Conditionally add write-access methods
    #[cfg(feature = "mcp_write_access")]
    let methods = {
        let mut m = methods;
        m.push(serde_json::json!({
            "method": "update_chapter_summary",
            "description": "Update the summary of a chapter (write access).",
            "parameters": { "chapter_id": "string", "summary": "string", "auth_token": "string" }
        }));
        m
    };

    let capabilities = serde_json::json!({
        "methods": methods
    });

    Ok(MCPResult {
        content: capabilities.to_string(),
        metadata: None,
    })
}

// Handle the mcp.prompts request
async fn handle_prompts() -> Result<MCPResult, MCPErrorResponse> {
    let prompts = serde_json::json!({
        "prompts": [
            "What happens in chapter 3 of the novel?",
            "Tell me about the protagonist character.",
            "Find all Q&A related to magic systems.",
            "Summarize the novel's plot.",
            "List all chapters with titles containing 'magic'.",
            "Retrieve character details for 'John Doe'."
        ]
    });

    Ok(MCPResult {
        content: prompts.to_string(),
        metadata: None,
    })
}

// Add write-access methods for updating summaries and cross-references
async fn handle_update_chapter_summary<T: DatabaseService>(
    db_service: &T,
    params: MCPParams,
) -> Result<MCPResult, MCPErrorResponse> {
    // Validate authentication token
    if !validate_auth_token(&params.options) {
        return Err(MCPErrorResponse {
            code: -32604, // Unauthorized
            message: "Invalid or missing authentication token".to_string(),
        });
    }

    // Extract chapter ID and new summary from params
    let chapter_id = params.options.get("chapter_id").and_then(|v| v.as_str());
    let new_summary = params.options.get("summary").and_then(|v| v.as_str());

    if chapter_id.is_none() || new_summary.is_none() {
        return Err(MCPErrorResponse {
            code: -32602, // Invalid params
            message: "Missing chapter_id or summary in request".to_string(),
        });
    }

    // Update the chapter summary in the database
    db_service
        .update_chapter_summary(chapter_id.unwrap(), new_summary.unwrap())
        .await
        .map_err(|e| MCPErrorResponse {
            code: -32603, // Internal error
            message: format!("Failed to update chapter summary: {}", e),
        })?;

    Ok(MCPResult {
        content: "Chapter summary updated successfully".to_string(),
        metadata: None,
    })
}

// Helper function to validate authentication tokens
fn validate_auth_token(options: &HashMap<String, serde_json::Value>) -> bool {
    if let Some(token) = options.get("auth_token").and_then(|v| v.as_str()) {
        // Replace with actual token validation logic
        token == "trusted_llm_token"
    } else {
        false
    }
}

// Format the database result into a more LLM-friendly text format
fn format_content_for_llm(data: &serde_json::Value, params: &SearchParams) -> String {
    if let serde_json::Value::Array(items) = data {
        if items.is_empty() {
            return format!("No results found for your query about {}.", params.collection);
        }

        // Format different types differently
        match params.collection.as_str() {
            "novels" => format_novels(items),
            "chapters" => format_chapters(items),
            "characters" => format_characters(items),
            "qa" => format_qa(items),
            _ => format!("Results for {}: {}", params.collection, serde_json::to_string_pretty(data).unwrap_or_default()),
        }
    } else {
        format!("Results: {}", serde_json::to_string_pretty(data).unwrap_or_default())
    }
}

// Helper functions to format different entity types in an LLM-friendly way
fn format_novels(items: &[serde_json::Value]) -> String {
    let mut result = format!("Found {} novels:\n\n", items.len());
    
    for (i, novel) in items.iter().enumerate() {
        if let Some(title) = novel.get("title").and_then(|t| t.as_str()) {
            let author = novel.get("author").and_then(|a| a.as_str()).unwrap_or("Unknown");
            let summary = novel.get("summary").and_then(|s| s.as_str()).unwrap_or("No summary available");
            
            result.push_str(&format!("{}. \"{}\" by {}\n", i + 1, title, author));
            result.push_str(&format!("   Summary: {}\n\n", summary));
        }
    }
    
    result
}

fn format_chapters(items: &[serde_json::Value]) -> String {
    let mut result = format!("Found {} chapters:\n\n", items.len());
    
    for chapter in items.iter() {
        if let Some(title) = chapter.get("title").and_then(|t| t.as_str()) {
            let number = chapter.get("number").and_then(|n| n.as_u64()).unwrap_or(0);
            let summary = chapter.get("summary").and_then(|s| s.as_str()).unwrap_or("No summary available");
            
            result.push_str(&format!("Chapter {}: {}\n", number, title));
            result.push_str(&format!("   Summary: {}\n\n", summary));
            
            // Add key points if available
            if let Some(key_points) = chapter.get("key_points").and_then(|k| k.as_array()) {
                if !key_points.is_empty() {
                    result.push_str("   Key points:\n");
                    for point in key_points {
                        if let Some(point_str) = point.as_str() {
                            result.push_str(&format!("   - {}\n", point_str));
                        }
                    }
                    result.push('\n');
                }
            }
        }
    }
    
    result
}

fn format_characters(items: &[serde_json::Value]) -> String {
    let mut result = format!("Found {} characters:\n\n", items.len());
    
    for (i, character) in items.iter().enumerate() {
        if let Some(name) = character.get("name").and_then(|n| n.as_str()) {
            let role = character.get("role").and_then(|r| r.as_str()).unwrap_or("Unknown role");
            let description = character.get("description").and_then(|d| d.as_str()).unwrap_or("No description available");
            
            result.push_str(&format!("{}. {} ({})\n", i + 1, name, role));
            result.push_str(&format!("   Description: {}\n", description));
            
            // Add key traits if available
            if let Some(traits) = character.get("key_traits").and_then(|t| t.as_array()) {
                if !traits.is_empty() {
                    result.push_str("   Key traits: ");
                    let traits_str: Vec<String> = traits
                        .iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect();
                    result.push_str(&traits_str.join(", "));
                    result.push('\n');
                }
            }
            
            // Add relationships if available
            if let Some(relationships) = character.get("relationships").and_then(|r| r.as_array()) {
                if !relationships.is_empty() {
                    result.push_str("   Relationships:\n");
                    for rel in relationships {
                        if let (Some(rel_name), Some(rel_type)) = (
                            rel.get("character_name").and_then(|n| n.as_str()),
                            rel.get("relationship_type").and_then(|t| t.as_str()),
                        ) {
                            result.push_str(&format!("   - {} ({})\n", rel_name, rel_type));
                        }
                    }
                }
            }
            
            result.push('\n');
        }
    }
    
    result
}

fn format_qa(items: &[serde_json::Value]) -> String {
    let mut result = format!("Found {} Q&A entries:\n\n", items.len());
    
    for (i, qa) in items.iter().enumerate() {
        if let (Some(question), Some(answer)) = (
            qa.get("question").and_then(|q| q.as_str()),
            qa.get("answer").and_then(|a| a.as_str()),
        ) {
            result.push_str(&format!("Q{}: {}\n", i + 1, question));
            result.push_str(&format!("A: {}\n\n", answer));
        }
    }
    
    result
}

// Custom error type for MCP errors
pub struct MCPErrorResponse {
    pub code: i32,
    pub message: String,
}

impl IntoResponse for MCPErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.code {
            -32700 => StatusCode::BAD_REQUEST, // Parse error
            -32600 => StatusCode::BAD_REQUEST, // Invalid Request
            -32601 => StatusCode::NOT_FOUND,   // Method not found
            -32602 => StatusCode::BAD_REQUEST, // Invalid params
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        let mcp_error = MCPError {
            code: self.code,
            message: self.message,
            data: None,
        };
        
        let body = Json(MCPResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: None,
            error: Some(mcp_error),
        });
        
        (status, body).into_response()
    }
}

// Implement From<anyhow::Error> for MCPErrorResponse to enable the ? operator
impl From<anyhow::Error> for MCPErrorResponse {
    fn from(error: anyhow::Error) -> Self {
        MCPErrorResponse {
            code: -32603, // Internal error
            message: format!("Internal error: {}", error),
        }
    }
}