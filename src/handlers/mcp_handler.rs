use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

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
    let search_params = QueryParser::parse_natural_language_query(&params.query);
    
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
    
    Ok(MCPResult {
        content,
        metadata: Some(serde_json::json!({
            "token_count": db_result.metadata.token_count,
            "query_time_ms": db_result.metadata.query_time_ms,
            "has_more": db_result.metadata.has_more,
            "next_page_token": db_result.metadata.next_page_token,
        })
        .as_object()
        .cloned()
        .unwrap_or_default()),
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
        content: format!("Chapter content for query: {}", params.query),
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
        content: format!("Character details for query: {}", params.query),
        metadata: None,
    })
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
    
    for (i, chapter) in items.iter().enumerate() {
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