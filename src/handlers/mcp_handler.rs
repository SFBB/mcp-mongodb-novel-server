use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use std::collections::HashMap;
use std::borrow::Cow;
use rmcp::{
    model::{
        CallToolResult, Content, PromptMessage, Implementation, 
        ListPromptsResult, ProtocolVersion, Prompt, ServerCapabilities, 
        ServerInfo, Tool, Annotated, PromptMessageRole,
        PromptMessageContent
    }, 
    service::{RequestContext, Service},
    Error as RmcpError, ServerHandler, RoleServer,
};
use serde_json::json;
use warp::Filter;

use axum::response::sse::{Sse, Event};
use futures_util::stream::{StreamExt, Stream}; // Make sure this is present
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;

use crate::models::{SearchParams, domain::MCPResponse};
use crate::services::db_service::DatabaseService;
use crate::utils::QueryParser;
use crate::mcp::conversion::ContentExt;
use crate::utils::query_parser;
use std::error::Error;
use serde_json::Value;
use uuid::Uuid;
use std::io::{BufRead, Write};

/// Server state for handling MCP requests
#[derive(Clone)]
pub struct ServerState<T: DatabaseService> {
    pub db_service: T,
}

/// MPC handler that implements the official RMCP SDK interface
#[derive(Clone)]
pub struct MpcHandler<T: DatabaseService + Clone + Send + Sync + 'static> {
    db_service: T
}

impl<T> MpcHandler<T> 
where 
    T: DatabaseService + Clone + Send + Sync + 'static 
{
    pub fn new(db_service: T) -> Self {
        MpcHandler { db_service }
    }
}

// Helper to create PromptMessageContent from string
fn create_content(text: &str) -> PromptMessageContent {
    PromptMessageContent::Text { 
        text: text.to_string()
    }
}

// Formatting functions to optimize responses for small context windows
fn format_novels(novels: &[serde_json::Value]) -> Value {
    if novels.is_empty() {
        return json!("No novels found matching your query.");
    }
    
    let formatted = novels.iter().enumerate().map(|(i, novel)| {
        let title = novel["title"].as_str().unwrap_or("Unknown title");
        let author = novel["author"].as_str().unwrap_or("Unknown author");
        let summary = novel["summary"].as_str().unwrap_or("No summary available");
        let id = novel["_id"].as_str().unwrap_or("");
        
        // Create a compact summary
        format!("{}. {} (by {}) - ID: {}\n   Summary: {}", 
                i+1, title, author, id, 
                truncate_text(summary, 150))
    }).collect::<Vec<String>>().join("\n\n");
    
    json!(formatted)
}

fn format_chapters(chapters: &[serde_json::Value]) -> Value {
    if chapters.is_empty() {
        return json!("No chapters found matching your query.");
    }
    
    let formatted = chapters.iter().enumerate().map(|(i, chapter)| {
        let title = chapter["title"].as_str().unwrap_or("Untitled");
        let novel_id = chapter["novel_id"].as_str().unwrap_or("Unknown novel");
        let id = chapter["_id"].as_str().unwrap_or("");
        let chapter_number = chapter["chapter_number"].as_u64().unwrap_or(0);
        
        format!("{}. Chapter {} - {} (ID: {})\n   Novel ID: {}", 
                i+1, chapter_number, title, id, novel_id)
    }).collect::<Vec<String>>().join("\n\n");
    
    json!(formatted)
}

fn format_characters(characters: &[serde_json::Value]) -> Value {
    if characters.is_empty() {
        return json!("No characters found matching your query.");
    }
    
    let formatted = characters.iter().enumerate().map(|(i, character)| {
        let name = character["name"].as_str().unwrap_or("Unknown");
        let novel_title = character["novel_title"].as_str().unwrap_or("Unknown novel");
        let id = character["_id"].as_str().unwrap_or("");
        let role = character["role"].as_str().unwrap_or("Unknown role");
        
        format!("{}. {} - {} in '{}' (ID: {})", 
                i+1, name, role, novel_title, id)
    }).collect::<Vec<String>>().join("\n\n");
    
    json!(formatted)
}

fn format_character_details(character: &serde_json::Value) -> Value {
    if character.is_null() {
        return json!("Character not found.");
    }
    
    let name = character["name"].as_str().unwrap_or("Unknown");
    let novel_title = character["novel_title"].as_str().unwrap_or("Unknown novel");
    let description = character["description"].as_str().unwrap_or("No description available");
    let role = character["role"].as_str().unwrap_or("Unknown role");
    let relationships = if let Some(rels) = character["relationships"].as_array() {
        rels.iter()
            .map(|rel| {
                let related_name = rel["name"].as_str().unwrap_or("Unknown");
                let relation_type = rel["relation_type"].as_str().unwrap_or("connected to");
                format!("- {} is {} {}", name, relation_type, related_name)
            })
            .collect::<Vec<String>>()
            .join("\n")
    } else {
        "No relationship information available".to_string()
    };
    
    let formatted = format!(
        "CHARACTER: {}\nROLE: {}\nNOVEL: {}\n\nDESCRIPTION:\n{}\n\nRELATIONSHIPS:\n{}",
        name, role, novel_title, description, relationships
    );
    
    json!(formatted)
}

fn format_qa(qa_entries: &[serde_json::Value]) -> Value {
    if qa_entries.is_empty() {
        return json!("No Q&A entries found matching your query.");
    }
    
    let formatted = qa_entries.iter().enumerate().map(|(i, qa)| {
        let question = qa["question"].as_str().unwrap_or("Unknown question");
        let answer = qa["answer"].as_str().unwrap_or("No answer available");
        let source = qa["source"].as_str().unwrap_or("Unknown source");
        
        format!("Q{}. {}\nA: {}\nSource: {}", 
                i+1, question, answer, source)
    }).collect::<Vec<String>>().join("\n\n");
    
    json!(formatted)
}

fn format_chapter_content(chapter: &serde_json::Value) -> Value {
    if chapter.is_null() {
        return json!("Chapter not found.");
    }
    
    let title = chapter["title"].as_str().unwrap_or("Untitled");
    let novel_title = chapter["novel_title"].as_str().unwrap_or("Unknown novel");
    let content = chapter["content"].as_str().unwrap_or("No content available");
    let chapter_number = chapter["chapter_number"].as_u64().unwrap_or(0);
    
    let content_summary = if content.len() > 2000 {
        format!("{}\n\n[Content truncated due to length. {} characters total]", 
                &content[0..2000], content.len())
    } else {
        content.to_string()
    };
    
    let formatted = format!(
        "NOVEL: {}\nCHAPTER {}: {}\n\nCONTENT:\n{}",
        novel_title, chapter_number, title, content_summary
    );
    
    json!(formatted)
}

fn format_all_results(results: &serde_json::Value) -> Value {
    let mut sections = Vec::new();
    
    if let Some(novels) = results["novels"].as_array() {
        if !novels.is_empty() {
            let formatted_novels = format!("NOVELS (top {} results):\n{}", 
                novels.len().min(3),
                novels.iter().take(3).enumerate().map(|(i, novel)| {
                    let title = novel["title"].as_str().unwrap_or("Unknown title");
                    let author = novel["author"].as_str().unwrap_or("Unknown author");
                    format!("{}. {} by {}", i+1, title, author)
                }).collect::<Vec<String>>().join("\n")
            );
            sections.push(formatted_novels);
        }
    }
    
    if let Some(characters) = results["characters"].as_array() {
        if !characters.is_empty() {
            let formatted_chars = format!("CHARACTERS (top {} results):\n{}", 
                characters.len().min(3),
                characters.iter().take(3).enumerate().map(|(i, char)| {
                    let name = char["name"].as_str().unwrap_or("Unknown");
                    let novel = char["novel_title"].as_str().unwrap_or("Unknown novel");
                    format!("{}. {} from {}", i+1, name, novel)
                }).collect::<Vec<String>>().join("\n")
            );
            sections.push(formatted_chars);
        }
    }
    
    if let Some(chapters) = results["chapters"].as_array() {
        if !chapters.is_empty() {
            let formatted_chapters = format!("CHAPTERS (top {} results):\n{}", 
                chapters.len().min(3),
                chapters.iter().take(3).enumerate().map(|(i, chapter)| {
                    let title = chapter["title"].as_str().unwrap_or("Untitled");
                    let novel = chapter["novel_title"].as_str().unwrap_or("Unknown novel");
                    format!("{}. {} from {}", i+1, title, novel)
                }).collect::<Vec<String>>().join("\n")
            );
            sections.push(formatted_chapters);
        }
    }
    
    if let Some(qa) = results["qa"].as_array() {
        if !qa.is_empty() {
            let formatted_qa = format!("Q&A (top {} results):\n{}", 
                qa.len().min(3),
                qa.iter().take(3).enumerate().map(|(i, q)| {
                    let question = q["question"].as_str().unwrap_or("Unknown question");
                    format!("{}. {}", i+1, truncate_text(question, 100))
                }).collect::<Vec<String>>().join("\n")
            );
            sections.push(formatted_qa);
        }
    }
    
    if sections.is_empty() {
        return json!("No results found matching your query.");
    }
    
    let total_count = results["novels"].as_array().map_or(0, |v| v.len()) +
                     results["characters"].as_array().map_or(0, |v| v.len()) +
                     results["chapters"].as_array().map_or(0, |v| v.len()) +
                     results["qa"].as_array().map_or(0, |v| v.len());
    
    let summary = format!(
        "Found {} results matching your query. Here's a summary:\n\n{}",
        total_count,
        sections.join("\n\n")
    );
    
    json!(summary)
}

// Helper function to truncate text with ellipsis
fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[0..max_length])
    }
}

impl<T: DatabaseService + Clone + Send + Sync + 'static> ServerHandler for MpcHandler<T> {
    /// Provide information about server capabilities
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "mcp_database".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("MongoDB MCP Server providing access to data with optimizations for small context windows (3k tokens).".into()),
        }
    }

    /// List available prompts for this server
    async fn list_prompts(
        &self,
        _request: rmcp::model::PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, RmcpError> {
        // Define example prompts for this database service
        let prompts = vec![
            Prompt {
                name: "search_novel".to_string(),
                description: Some("Search for novels matching specific criteria".to_string()),
                arguments: None,
            },
            Prompt {
                name: "character_details".to_string(),
                description: Some("Get detailed information about a character".to_string()),
                arguments: None,
            },
            Prompt {
                name: "chapter_summary".to_string(), 
                description: Some("Get a summary of a specific chapter".to_string()),
                arguments: None,
            },
        ];

        Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
        })
    }

    /// Get a specific prompt by name
    async fn get_prompt(
        &self,
        request: rmcp::model::GetPromptRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::GetPromptResult, RmcpError> {
        // Return a specific prompt based on name
        match request.name.as_str() {
            "search_novel" => {
                Ok(rmcp::model::GetPromptResult {
                    description: Some("Search for novels matching specific criteria".to_string()),
                    messages: vec![
                        PromptMessage {
                            role: PromptMessageRole::User,
                            content: create_content("Find novels about magic and dragons"),
                        }
                    ],
                })
            },
            "character_details" => {
                Ok(rmcp::model::GetPromptResult {
                    description: Some("Get detailed information about a character".to_string()),
                    messages: vec![
                        PromptMessage {
                            role: PromptMessageRole::User,
                            content: create_content("Tell me about the protagonist of the novel \"Dragon's Journey\""),
                        }
                    ],
                })
            },
            "chapter_summary" => {
                Ok(rmcp::model::GetPromptResult {
                    description: Some("Get a summary of a specific chapter".to_string()),
                    messages: vec![
                        PromptMessage {
                            role: PromptMessageRole::User,
                            content: create_content("Summarize chapter 5 of \"The Lost Kingdom\""),
                        }
                    ],
                })
            },
            _ => {
                Err(RmcpError::invalid_params("Unknown method", None))
            }
        }
    }

    /// List available tools for this server
    async fn list_tools(
        &self, 
        _request: rmcp::model::PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, RmcpError> {
        use rmcp::model::{Tool, ListToolsResult};
        use std::borrow::Cow;
        
        let tool_schema = std::sync::Arc::new(serde_json::to_value({
            let mut schema = serde_json::Map::new();
            schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            
            let mut properties = serde_json::Map::new();
            
            let mut query_prop = serde_json::Map::new();
            query_prop.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            query_prop.insert("description".to_string(), serde_json::Value::String("Natural language query to search the database".to_string()));
            
            let mut collection_prop = serde_json::Map::new();
            collection_prop.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            collection_prop.insert("description".to_string(), serde_json::Value::String("Type of data to query: novels, chapters, characters, or qa".to_string()));
            
            let mut enum_values = serde_json::Value::Array(vec![]);
            if let serde_json::Value::Array(ref mut arr) = enum_values {
                arr.push(serde_json::Value::String("novels".to_string()));
                arr.push(serde_json::Value::String("chapters".to_string()));
                arr.push(serde_json::Value::String("characters".to_string()));
                arr.push(serde_json::Value::String("qa".to_string()));
            }
            collection_prop.insert("enum".to_string(), enum_values);
            
            properties.insert("query".to_string(), serde_json::Value::Object(query_prop));
            properties.insert("collection".to_string(), serde_json::Value::Object(collection_prop));
            
            schema.insert("properties".to_string(), serde_json::Value::Object(properties));
            
            let required = serde_json::Value::Array(vec![serde_json::Value::String("query".to_string())]);
            schema.insert("required".to_string(), required);
            
            schema
        }).unwrap_or_default().as_object().unwrap().clone());

        // Define the database query tool
        let query_tool = Tool {
            name: Cow::from("query_database"),
            description: Cow::from("Query the database using natural language"),
            input_schema: tool_schema,
        };
        
        // Create chapter tool schema
        let chapter_schema = std::sync::Arc::new(serde_json::to_value({
            let mut schema = serde_json::Map::new();
            schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            
            let mut properties = serde_json::Map::new();
            let mut chapter_id_prop = serde_json::Map::new();
            chapter_id_prop.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            chapter_id_prop.insert("description".to_string(), serde_json::Value::String("ID of the chapter to retrieve".to_string()));
            properties.insert("chapter_id".to_string(), serde_json::Value::Object(chapter_id_prop));
            
            schema.insert("properties".to_string(), serde_json::Value::Object(properties));
            let required = serde_json::Value::Array(vec![serde_json::Value::String("chapter_id".to_string())]);
            schema.insert("required".to_string(), required);
            
            schema
        }).unwrap_or_default().as_object().unwrap().clone());
        
        // Define the chapter content tool
        let chapter_tool = Tool {
            name: Cow::from("get_chapter_content"),
            description: Cow::from("Retrieve the content of a specific chapter"),
            input_schema: chapter_schema,
        };
        
        // Define the character details tool with proper schema format
        let character_schema = std::sync::Arc::new(serde_json::to_value({
            let mut schema = serde_json::Map::new();
            schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            
            let mut properties = serde_json::Map::new();
            let mut character_id_prop = serde_json::Map::new();
            character_id_prop.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            character_id_prop.insert("description".to_string(), serde_json::Value::String("ID of the character to retrieve".to_string()));
            properties.insert("character_id".to_string(), serde_json::Value::Object(character_id_prop));
            
            schema.insert("properties".to_string(), serde_json::Value::Object(properties));
            let required = serde_json::Value::Array(vec![serde_json::Value::String("character_id".to_string())]);
            schema.insert("required".to_string(), required);
            
            schema
        }).unwrap_or_default().as_object().unwrap().clone());
        
        let character_tool = Tool {
            name: Cow::from("get_character_details"),
            description: Cow::from("Retrieve detailed information about a character"),
            input_schema: character_schema,
        };
        
        // Create regex QA tool schema
        let regex_qa_schema = std::sync::Arc::new(serde_json::to_value({
            let mut schema = serde_json::Map::new();
            schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            
            let mut properties = serde_json::Map::new();
            let mut regex_prop = serde_json::Map::new();
            regex_prop.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            regex_prop.insert("description".to_string(), serde_json::Value::String("Regular expression to match in Q&A entries".to_string()));
            properties.insert("regex_pattern".to_string(), serde_json::Value::Object(regex_prop));
            
            schema.insert("properties".to_string(), serde_json::Value::Object(properties));
            let required = serde_json::Value::Array(vec![serde_json::Value::String("regex_pattern".to_string())]);
            schema.insert("required".to_string(), required);
            
            schema
        }).unwrap_or_default().as_object().unwrap().clone());
        
        let regex_qa_tool = Tool {
            name: Cow::from("query_qa_regex"),
            description: Cow::from("Search Q&A entries using a regex pattern"),
            input_schema: regex_qa_schema,
        };
        
        // Create chapter regex tool schema
        let regex_chapter_schema = std::sync::Arc::new(serde_json::to_value({
            let mut schema = serde_json::Map::new();
            schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            
            let mut properties = serde_json::Map::new();
            let mut regex_prop = serde_json::Map::new();
            regex_prop.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            regex_prop.insert("description".to_string(), serde_json::Value::String("Regular expression to match in chapter titles or content".to_string()));
            properties.insert("regex_pattern".to_string(), serde_json::Value::Object(regex_prop));
            
            schema.insert("properties".to_string(), serde_json::Value::Object(properties));
            let required = serde_json::Value::Array(vec![serde_json::Value::String("regex_pattern".to_string())]);
            schema.insert("required".to_string(), required);
            
            schema
        }).unwrap_or_default().as_object().unwrap().clone());
        
        let regex_chapter_tool = Tool {
            name: Cow::from("query_chapter_regex"),
            description: Cow::from("Search chapters using a regex pattern"),
            input_schema: regex_chapter_schema,
        };
        
        // Create character regex tool schema
        let regex_character_schema = std::sync::Arc::new(serde_json::to_value({
            let mut schema = serde_json::Map::new();
            schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));
            
            let mut properties = serde_json::Map::new();
            let mut regex_prop = serde_json::Map::new();
            regex_prop.insert("type".to_string(), serde_json::Value::String("string".to_string()));
            regex_prop.insert("description".to_string(), serde_json::Value::String("Regular expression to match in character names or descriptions".to_string()));
            properties.insert("regex_pattern".to_string(), serde_json::Value::Object(regex_prop));
            
            schema.insert("properties".to_string(), serde_json::Value::Object(properties));
            let required = serde_json::Value::Array(vec![serde_json::Value::String("regex_pattern".to_string())]);
            schema.insert("required".to_string(), required);
            
            schema
        }).unwrap_or_default().as_object().unwrap().clone());
        
        let regex_character_tool = Tool {
            name: Cow::from("query_character_regex"),
            description: Cow::from("Search characters using a regex pattern"),
            input_schema: regex_character_schema,
        };
        
        // Create the list of tools
        let tools = vec![
            query_tool,
            chapter_tool,
            character_tool,
            regex_qa_tool,
            regex_chapter_tool,
            regex_character_tool,
        ];
        
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    /// Execute a specific tool
    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, RmcpError> {
        let tool_name = request.name.as_ref();
        let args = request.arguments.unwrap_or_default();
        
        // Dispatch to the appropriate handler based on tool name
        match tool_name {
            "query_database" => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RmcpError::invalid_params("Missing 'query' parameter", None))?;
                
                let collection = args.get("collection")
                    .and_then(|v| v.as_str())
                    .unwrap_or("all");
                
                // Parse the natural language query into search parameters
                let search_params = QueryParser::parse_natural_language_query(query);
                
                // Search the database with the parsed parameters
                let result = match collection {
                    "novels" => self.db_service.search_novels(&search_params).await,
                    "chapters" => self.db_service.search_chapters(&search_params).await,
                    "characters" => self.db_service.search_characters(&search_params).await,
                    "qa" => self.db_service.search_qa(&search_params).await,
                    _ => {
                        let result = self.db_service.search_all(&search_params).await
                            .map_err(|e| RmcpError::internal_error(format!("Database error: {}", e), None))?;
                        
                        // Convert from Value to MCPResponse
                        let content = format_all_results(&result);
                        return Ok(CallToolResult {
                            content: vec![Content::from_raw(content.to_string())],
                            is_error: Some(false),
                        });
                    },
                }.map_err(|e| RmcpError::internal_error(format!("Database error: {}", e), None))?;
                
                // Format the result in a token-efficient manner
                let content_str = match collection {
                    "novels" => format_novels(result.data.as_array().unwrap()),
                    "chapters" => format_chapters(result.data.as_array().unwrap()),
                    "characters" => format_characters(result.data.as_array().unwrap()),
                    "qa" => format_qa(result.data.as_array().unwrap()),
                    _ => format_all_results(&result.data),
                };
                
                Ok(CallToolResult {
                    content: vec![Content::from_raw(content_str.to_string())],
                    is_error: Some(false),
                })
            },
            "get_chapter_content" => {
                let chapter_id = args.get("chapter_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RmcpError::invalid_params("Missing 'chapter_id' parameter", None))?;
                
                // Get chapter content from database
                let result = self.db_service.get_chapter_content(chapter_id).await
                    .map_err(|e| RmcpError::internal_error(format!("Database error: {}", e), None))?;
                
                Ok(CallToolResult {
                    content: vec![Content::from_raw(result.unwrap())],
                    is_error: Some(false),
                })
            },
            "get_character_details" => {
                let character_id = args.get("character_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RmcpError::invalid_params("Missing 'character_id' parameter", None))?;
                
                // Get character details from database
                let result = self.db_service.get_character_details(character_id).await
                    .map_err(|e| RmcpError::internal_error(format!("Database error: {}", e), None))?;
                
                // Format the character details
                if let Some(character) = result {
                    let character_json = serde_json::to_value(&character)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    
                    let formatted = format_character_details(&character_json);
                    
                    Ok(CallToolResult {
                        content: vec![Content::from_raw(formatted.to_string())],
                        is_error: Some(false),
                    })
                } else {
                    Ok(CallToolResult {
                        content: vec![Content::from_raw(format!("No character found with ID: {}", character_id))],
                        is_error: Some(false),
                    })
                }
            },
            "query_qa_regex" => {
                let regex_pattern = args.get("regex_pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RmcpError::invalid_params("Missing 'regex_pattern' parameter", None))?;
                
                // Search Q&A entries with regex
                let result = self.db_service.search_qa_by_regex(regex_pattern).await
                    .map_err(|e| RmcpError::internal_error(format!("Database error: {}", e), None))?;
                
                // Format the results
                let formatted = format_qa(&result);
                
                Ok(CallToolResult {
                    content: vec![Content::from_raw(formatted.to_string())],
                    is_error: Some(false),
                })
            },
            "query_chapter_regex" => {
                let regex_pattern = args.get("regex_pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RmcpError::invalid_params("Missing 'regex_pattern' parameter", None))?;
                
                // Search chapters with regex
                let result = self.db_service.search_chapters_by_regex(regex_pattern).await
                    .map_err(|e| RmcpError::internal_error(format!("Database error: {}", e), None))?;
                
                // Format the results
                let formatted = format_chapters(&result);
                
                Ok(CallToolResult {
                    content: vec![Content::from_raw(formatted.to_string())],
                    is_error: Some(false),
                })
            },
            "query_character_regex" => {
                let regex_pattern = args.get("regex_pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RmcpError::invalid_params("Missing 'regex_pattern' parameter", None))?;
                
                // Search characters with regex
                let result = self.db_service.search_characters_by_regex(regex_pattern).await
                    .map_err(|e| RmcpError::internal_error(format!("Database error: {}", e), None))?;
                
                // Format the results
                let formatted = format_characters(&result);
                
                Ok(CallToolResult {
                    content: vec![Content::from_raw(formatted.to_string())],
                    is_error: Some(false),
                })
            },
            _ => {
                Err(RmcpError::invalid_params("Unknown method", None))
            }
        }
    }
}

pub async fn sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(1)))
        .enumerate()
        .map(|(i, _)| Ok(Event::default().data(format!("tick: {}", i))));
    Sse::new(stream)
}

// // Add this function to create a filter that adds server state to the request
// fn with_server_state<T: DatabaseService + Clone + Send + Sync + 'static>(
//     state: ServerState<T>
// ) -> impl Filter<Extract = (ServerState<T>,), Error = std::convert::Infallible> + Clone {
//     warp::any().map(move || state.clone())
// }

// async fn handle_mcp_request<T: DatabaseService + Clone + Send + Sync + 'static>(
//     request: serde_json::Value, 
//     state: ServerState<T>
// ) -> Result<impl warp::Reply, warp::Rejection> {
//     let mpc_handler = MpcHandler {
//         db_service: state.db_service
//     };
    
//     // Process the request manually since HttpService isn't available
//     let response = match mpc_handler.handle_request(request).await {
//         Ok(resp) => resp,
//         Err(err) => json!({
//             "error": {
//                 "code": -32603,
//                 "message": format!("Internal error: {}", err)
//             }
//         }),
//     };
    
//     Ok(warp::reply::json(&response))
// }

// // Fix StdIO handler to manually implement the service since StdioService isn't available
// pub async fn run_stdio_mcp_server<T: DatabaseService + Clone + Send + Sync + 'static>(
//     state: ServerState<T>
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let mpc_handler = MpcHandler {
//         db_service: state.db_service.clone(),
//     };
    
//     // Manual implementation of StdIO service
//     let stdin = std::io::stdin();
//     let mut stdin_lock = stdin.lock();
//     let stdout = std::io::stdout();
//     let mut stdout_lock = stdout.lock();
    
//     let mut buffer = String::new();
    
//     loop {
//         buffer.clear();
//         match stdin_lock.read_line(&mut buffer) {
//             Ok(0) => break, // EOF
//             Ok(_) => {
//                 let request: serde_json::Value = match serde_json::from_str(&buffer) {
//                     Ok(req) => req,
//                     Err(e) => {
//                         let error_response = json!({
//                             "error": {
//                                 "code": -32700,
//                                 "message": format!("Parse error: {}", e)
//                             }
//                         });
//                         serde_json::to_writer(&mut stdout_lock, &error_response)?;
//                         writeln!(&mut stdout_lock)?;
//                         continue;
//                     }
//                 };
                
//                 // Process the request
//                 let response = match mpc_handler.handle_request(request).await {
//                     Ok(resp) => resp,
//                     Err(e) => {
//                         json!({
//                             "error": {
//                                 "code": -32603,
//                                 "message": format!("Internal error: {}", e)
//                             }
//                         })
//                     }
//                 };
                
//                 // Write the response
//                 serde_json::to_writer(&mut stdout_lock, &response)?;
//                 writeln!(&mut stdout_lock)?;
//             }
//             Err(e) => {
//                 eprintln!("Error reading from stdin: {}", e);
//                 break;
//             }
//         }
//     }
    
//     Ok(())
// }