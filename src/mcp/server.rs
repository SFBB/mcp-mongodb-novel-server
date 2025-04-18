// src/mcp/server.rs
use rmcp::{model::{ServerInfo, CallToolResult, RawContent, Annotated}, ServerHandler, tool, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::services::DatabaseService;

#[derive(Debug, Clone)]
pub struct MCPDatabaseServer<T: DatabaseService + Clone + Send + Sync + 'static> {
    db_service: Arc<T>,
}

impl<T: DatabaseService + Clone + Send + Sync + 'static> MCPDatabaseServer<T> {
    pub fn new(db_service: T) -> Self {
        Self {
            db_service: Arc::new(db_service),
        }
    }
}

// Implement general MCP server handling
impl<T: DatabaseService + Clone + Send + Sync + 'static> ServerHandler for MCPDatabaseServer<T> {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("MongoDB MCP Server providing access to data with optimizations for small context windows.".into()),
            ..Default::default()
        }
    }
}

// Making formatting functions public so we can use them in the server
pub mod formatting {
    use crate::models::SearchParams;

    pub fn format_content_for_llm(data: &serde_json::Value, params: &SearchParams) -> String {
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
    
    pub fn format_novels(items: &[serde_json::Value]) -> String {
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
    
    pub fn format_chapters(items: &[serde_json::Value]) -> String {
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
    
    pub fn format_characters(items: &[serde_json::Value]) -> String {
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
    
    pub fn format_qa(items: &[serde_json::Value]) -> String {
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
}

// Query parameter types
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct QueryRequest {
    #[schemars(description = "The natural language query to execute")]
    pub query: String,
    
    #[schemars(description = "Optional parameters for the query")]
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct RegexQueryRequest {
    #[schemars(description = "The regex pattern to search for")]
    pub regex_pattern: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct ChapterContentRequest {
    #[schemars(description = "The ID of the chapter to retrieve")]
    pub chapter_id: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct CharacterDetailsRequest {
    #[schemars(description = "The ID of the character to retrieve")]
    pub character_id: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct UpdateChapterSummaryRequest {
    #[schemars(description = "The ID of the chapter to update")]
    pub chapter_id: String,
    
    #[schemars(description = "The new summary text")]
    pub summary: String,
    
    #[schemars(description = "Authentication token for write access")]
    pub auth_token: String,
}

// MCP tool implementations - implementing each separately to avoid macro issues
impl<T: DatabaseService + Clone + Send + Sync + 'static> MCPDatabaseServer<T> {
    #[tool(description = "Execute a natural language query against the database")]
    pub async fn query(&self, #[tool(param)] query: String) -> Result<String, String> {
        use crate::utils::QueryParser;
        
        // Parse the natural language query into structured params
        let search_params = QueryParser::parse_natural_language_query(&query);
        
        // Execute the appropriate search based on collection type
        let db_response = match search_params.collection.as_str() {
            "novels" => self.db_service.as_ref().search_novels(&search_params).await,
            "chapters" => self.db_service.as_ref().search_chapters(&search_params).await,
            "characters" => self.db_service.as_ref().search_characters(&search_params).await,
            "qa" => self.db_service.as_ref().search_qa(&search_params).await,
            _ => {
                return Err(format!("Unknown collection type: {}", search_params.collection));
            }
        };
        
        // Handle database errors
        let db_result = match db_response {
            Ok(result) => result,
            Err(e) => {
                return Err(format!("Database error: {}", e));
            }
        };
        
        // Format result for LLM consumption
        let content = formatting::format_content_for_llm(&db_result.data, &search_params);
        
        Ok(content)
    }
    
    #[tool(description = "Retrieve specific chapter content by ID")]
    pub async fn get_chapter_content(&self, #[tool(param)] chapter_id: String) -> Result<String, String> {
        // Cast to the concrete type MongoDBService that we know implements these methods
        // This is a workaround for the trait not having these methods
        let db_service = self.db_service.as_ref();
        
        // We'll need to use a more flexible approach, like looking up chapters in the DB directly
        let chapters = db_service.search_chapters_by_regex(&format!("^{}$", &chapter_id)).await
            .map_err(|e| format!("Failed to retrieve chapter: {}", e))?;
            
        if let Some(chapter) = chapters.get(0) {
            Ok(serde_json::to_string_pretty(chapter).unwrap_or_default())
        } else {
            Err(format!("Chapter with ID {} not found", chapter_id))
        }
    }
    
    #[tool(description = "Retrieve detailed character information by ID")]
    pub async fn get_character_details(&self, #[tool(param)] character_id: String) -> Result<String, String> {
        // Cast to the concrete type MongoDBService that we know implements these methods
        // This is a workaround for the trait not having these methods
        let db_service = self.db_service.as_ref();
        
        // We'll need to use a more flexible approach, like looking up characters in the DB directly
        let characters = db_service.search_characters_by_regex(&format!("^{}$", &character_id)).await
            .map_err(|e| format!("Failed to retrieve character: {}", e))?;
            
        if let Some(character) = characters.get(0) {
            Ok(serde_json::to_string_pretty(character).unwrap_or_default())
        } else {
            Err(format!("Character with ID {} not found", character_id))
        }
    }
    
    #[tool(description = "Search Q&A entries using regex pattern")]
    pub async fn query_qa_regex(&self, #[tool(param)] regex_pattern: String) -> Result<String, String> {
        let qa_entries = self.db_service.as_ref().search_qa_by_regex(&regex_pattern).await
            .map_err(|e| format!("Failed to search Q&A entries: {}", e))?;
            
        let formatted = formatting::format_qa(&qa_entries);
        Ok(formatted)
    }
    
    #[tool(description = "Search chapters using regex pattern")]
    pub async fn query_chapter_regex(&self, #[tool(param)] regex_pattern: String) -> Result<String, String> {
        let chapters = self.db_service.as_ref().search_chapters_by_regex(&regex_pattern).await
            .map_err(|e| format!("Failed to search chapters: {}", e))?;
            
        let formatted = formatting::format_chapters(&chapters);
        Ok(formatted)
    }
    
    #[tool(description = "Search characters using regex pattern")]
    pub async fn query_character_regex(&self, #[tool(param)] regex_pattern: String) -> Result<String, String> {
        let characters = self.db_service.as_ref().search_characters_by_regex(&regex_pattern).await
            .map_err(|e| format!("Failed to search characters: {}", e))?;
            
        let formatted = formatting::format_characters(&characters);
        Ok(formatted)
    }
    
    #[cfg(feature = "mcp_write_access")]
    #[tool(description = "Update a chapter's summary (requires write access)")]
    pub async fn update_chapter_summary(
        &self, 
        #[tool(param)] chapter_id: String, 
        #[tool(param)] summary: String, 
        #[tool(param)] auth_token: String
    ) -> Result<String, String> {
        // Validate authentication token
        if auth_token != "trusted_llm_token" {
            return Err("Invalid or missing authentication token".to_string());
        }
        
        // Update the chapter summary in the database
        self.db_service.as_ref()
            .update_chapter_summary(&chapter_id, &summary)
            .await
            .map_err(|e| format!("Failed to update chapter summary: {}", e))?;
        
        Ok("Chapter summary updated successfully".to_string())
    }
}

// Implement direct methods that can be called from our HTTP handler
impl<T: DatabaseService + Clone + Send + Sync + 'static> MCPDatabaseServer<T> {
    // This method handles direct query requests from HTTP handler
    pub async fn handle_query(&self, query: &str) -> Result<CallToolResult, Error> {
        match self.query(query.to_string()).await {
            Ok(content) => {
                Ok(CallToolResult {
                    content: vec![Annotated::new(RawContent::text(content), None)],
                    is_error: None,
                })
            },
            Err(e) => {
                Err(Error::invalid_params(e, None))
            }
        }
    }
    
    // Direct method for chapter content
    pub async fn handle_chapter_content(&self, chapter_id: &str) -> Result<CallToolResult, Error> {
        // Reuse our existing tool implementation
        match self.get_chapter_content(chapter_id.to_string()).await {
            Ok(content) => {
                Ok(CallToolResult {
                    content: vec![Annotated::new(RawContent::text(content), None)],
                    is_error: None,
                })
            },
            Err(e) => {
                Err(Error::invalid_params(e, None))
            }
        }
    }
    
    // Direct method for character details
    pub async fn handle_character_details(&self, character_id: &str) -> Result<CallToolResult, Error> {
        // Reuse our existing tool implementation
        match self.get_character_details(character_id.to_string()).await {
            Ok(content) => {
                Ok(CallToolResult {
                    content: vec![Annotated::new(RawContent::text(content), None)],
                    is_error: None,
                })
            },
            Err(e) => {
                Err(Error::invalid_params(e, None))
            }
        }
    }
    
    // Direct method for Q&A regex
    pub async fn handle_qa_regex(&self, regex: &str) -> Result<CallToolResult, Error> {
        match self.query_qa_regex(regex.to_string()).await {
            Ok(content) => {
                Ok(CallToolResult {
                    content: vec![Annotated::new(RawContent::text(content), None)],
                    is_error: None,
                })
            },
            Err(e) => {
                Err(Error::invalid_params(e, None))
            }
        }
    }
    
    // Direct method for chapter regex
    pub async fn handle_chapter_regex(&self, regex: &str) -> Result<CallToolResult, Error> {
        match self.query_chapter_regex(regex.to_string()).await {
            Ok(content) => {
                Ok(CallToolResult {
                    content: vec![Annotated::new(RawContent::text(content), None)],
                    is_error: None,
                })
            },
            Err(e) => {
                Err(Error::invalid_params(e, None))
            }
        }
    }
    
    // Direct method for character regex
    pub async fn handle_character_regex(&self, regex: &str) -> Result<CallToolResult, Error> {
        match self.query_character_regex(regex.to_string()).await {
            Ok(content) => {
                Ok(CallToolResult {
                    content: vec![Annotated::new(RawContent::text(content), None)],
                    is_error: None,
                })
            },
            Err(e) => {
                Err(Error::invalid_params(e, None))
            }
        }
    }
    
    // Direct method for updating chapter summary
    pub async fn handle_chapter_summary_update(&self, chapter_id: &str, summary: &str) -> Result<CallToolResult, Error> {
        #[cfg(feature = "mcp_write_access")]
        {
            // Assuming a default token for simplicity - in production would use proper auth
            let auth_token = "trusted_llm_token".to_string();
            match self.update_chapter_summary(chapter_id.to_string(), summary.to_string(), auth_token).await {
                Ok(content) => {
                    Ok(CallToolResult {
                        content: vec![Annotated::new(RawContent::text(content), None)],
                        is_error: None,
                    })
                },
                Err(e) => {
                    Err(Error::invalid_params(e, None))
                }
            }
        }
        
        #[cfg(not(feature = "mcp_write_access"))]
        {
            // Create an error with a permission denied code (403)
            Err(Error::new(
                rmcp::model::ErrorCode(403), // Use numerical code for Permission Denied
                "Write access not enabled in this build".to_string(),
                None
            ))
        }
    }
}