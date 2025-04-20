use anyhow::Result;
use std::sync::Arc;
use async_trait::async_trait;
use mongodb::{
    bson::{doc, Document, oid::ObjectId},
    options::FindOptions,
};
use std::time::{Duration, Instant};
use futures::TryStreamExt; // Add the TryStreamExt trait

use crate::db::DatabaseConnection;
use crate::models::{Chapter, Character, MCPResponse, Novel, QA, ResponseMetadata, SearchParams};

#[async_trait]
pub trait DatabaseService {
    async fn search_novels(&self, params: &SearchParams) -> Result<MCPResponse>;
    async fn search_chapters(&self, params: &SearchParams) -> Result<MCPResponse>;
    async fn search_characters(&self, params: &SearchParams) -> Result<MCPResponse>;
    async fn search_qa(&self, params: &SearchParams) -> Result<MCPResponse>;
    async fn search_all(&self, params: &SearchParams) -> Result<serde_json::Value>;
    async fn search_qa_by_regex(&self, regex_pattern: &str) -> Result<Vec<serde_json::Value>>;
    async fn search_chapters_by_regex(&self, regex_pattern: &str) -> Result<Vec<serde_json::Value>>;
    async fn search_characters_by_regex(&self, regex_pattern: &str) -> Result<Vec<serde_json::Value>>;
    async fn get_chapter_content(&self, chapter_id: &str) -> Result<Option<String>>;
    async fn get_character_details(&self, character_id: &str) -> Result<Option<Character>>;
    async fn update_chapter_summary(&self, chapter_id: &str, new_summary: &str) -> Result<()>;
}

#[derive(Clone)]
pub struct MongoDBService {
    db: DatabaseConnection,
}

impl MongoDBService {
    pub async fn new() -> Result<Self> {
        let db = DatabaseConnection::new().await?;
        Ok(Self { db })
    }

    // Helper function to estimate token count of JSON data
    fn estimate_token_count(data: &serde_json::Value) -> u32 {
        // Very rough estimate: 1 token ≈ 4 chars in English text
        let json_string = serde_json::to_string(data).unwrap_or_default();
        (json_string.len() as u32 + 3) / 4
    }

    // Helper to convert MongoDB documents to JSON
    async fn format_response<T>(&self, 
        data: Vec<T>, 
        query_time: Duration, 
        has_more: bool,
        _limit: Option<u32>
    ) -> MCPResponse 
    where 
        T: serde::Serialize 
    {
        let data_json = serde_json::to_value(data).unwrap_or(serde_json::Value::Array(vec![]));
        
        // Estimate token count
        let token_count = Self::estimate_token_count(&data_json);
        
        // Create next page token if there are more results
        let next_page_token = if has_more {
            // In a real implementation, we would create a proper pagination token
            Some(format!("page_token_{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()))
        } else {
            None
        };
        
        MCPResponse {
            status: "success".to_string(),
            data: data_json,
            metadata: ResponseMetadata {
                token_count: Some(token_count),
                query_time_ms: query_time.as_millis() as u64,
                has_more,
                next_page_token,
            },
        }
    }
}

// Helper function to build a search filter based on keywords
fn build_text_search_filter(keywords: &[String]) -> Document {
    if keywords.is_empty() {
        return doc! {};
    }

    // Join keywords for text search
    let search_text = keywords.join(" ");
    
    doc! {
        "$text": {
            "$search": search_text
        }
    }
}

#[async_trait]
impl DatabaseService for Arc<MongoDBService> {
    async fn search_novels(&self, params: &SearchParams) -> Result<MCPResponse> {
        let start = Instant::now();
        
        // Build filter from search params
        let mut filter = build_text_search_filter(&params.keywords);
        
        // Add additional filters if provided
        if let Some(filters) = &params.filters {
            if let Some(tags) = &filters.tags {
                filter.insert("tags", doc! { "$in": tags });
            }
        }
        
        // Set limit for small context window optimization
        let limit = params.limit.unwrap_or(5);
        let options = FindOptions::builder()
            .limit(limit as i64 + 1) // Fetch one extra to check if there are more
            .build();
        
        // Execute query
        let collection = self.db.get_collection::<Novel>("novels");
        let mut cursor = collection.find(filter, options).await?;
        
        // Collect results
        let mut novels = Vec::new();
        while let Some(novel) = cursor.try_next().await? {
            novels.push(novel);
        }
        
        // Check if there are more results
        let has_more = novels.len() > limit as usize;
        if has_more {
            novels.pop(); // Remove the extra item
        }
        
        let query_time = start.elapsed();
        Ok(self.format_response(novels, query_time, has_more, Some(limit)).await)
    }
    
    async fn search_chapters(&self, params: &SearchParams) -> Result<MCPResponse> {
        let start = Instant::now();
        
        // Build filter from search params
        let mut filter = build_text_search_filter(&params.keywords);
        
        // Add additional filters if provided
        if let Some(filters) = &params.filters {
            if let Some(novel_id) = &filters.novel_id {
                // Convert string ID to ObjectId if possible
                if let Ok(oid) = ObjectId::parse_str(novel_id) {
                    filter.insert("novel_id", oid);
                }
            }
        }
        
        // Set limit for small context window optimization
        let limit = params.limit.unwrap_or(3);
        let options = FindOptions::builder()
            .limit(limit as i64 + 1) // Fetch one extra to check if there are more
            .sort(doc! { "number": 1 }) // Sort by chapter number
            .build();
        
        // Execute query
        let collection = self.db.get_collection::<Chapter>("chapters");
        let mut cursor = collection.find(filter, options).await?;
        
        // Collect results - only include summaries and key points for small context
        let mut chapters = Vec::new();
        while let Some(chapter) = cursor.try_next().await? {
            // Create a chapter with content excluded to save tokens
            let compact_chapter = Chapter {
                id: chapter.id,
                novel_id: chapter.novel_id,
                number: chapter.number,
                title: chapter.title,
                summary: chapter.summary,
                key_points: chapter.key_points,
                content: None, // Exclude full content to save tokens
            };
            chapters.push(compact_chapter);
        }
        
        // Check if there are more results
        let has_more = chapters.len() > limit as usize;
        if has_more {
            chapters.pop(); // Remove the extra item
        }
        
        let query_time = start.elapsed();
        Ok(self.format_response(chapters, query_time, has_more, Some(limit)).await)
    }
    
    async fn search_characters(&self, params: &SearchParams) -> Result<MCPResponse> {
        let start = Instant::now();
        
        // Build filter from search params
        let mut filter = build_text_search_filter(&params.keywords);
        
        // Add additional filters if provided
        if let Some(filters) = &params.filters {
            if let Some(novel_id) = &filters.novel_id {
                // Convert string ID to ObjectId if possible
                if let Ok(oid) = ObjectId::parse_str(novel_id) {
                    filter.insert("novel_id", oid);
                }
            }
            
            if let Some(character_name) = &filters.character_name {
                filter.insert("name", doc! { "$regex": character_name, "$options": "i" });
            }
        }
        
        // Set limit for small context window optimization
        let limit = params.limit.unwrap_or(5);
        let options = FindOptions::builder()
            .limit(limit as i64 + 1) // Fetch one extra to check if there are more
            .sort(doc! { "name": 1 }) // Sort by character name
            .build();
        
        // Execute query
        let collection = self.db.get_collection::<Character>("characters");
        let mut cursor = collection.find(filter, options).await?;
        
        // Collect results
        let mut characters = Vec::new();
        while let Some(character) = cursor.try_next().await? {
            characters.push(character);
        }
        
        // Check if there are more results
        let has_more = characters.len() > limit as usize;
        if has_more {
            characters.pop(); // Remove the extra item
        }
        
        let query_time = start.elapsed();
        Ok(self.format_response(characters, query_time, has_more, Some(limit)).await)
    }
    
    async fn search_qa(&self, params: &SearchParams) -> Result<MCPResponse> {
        let start = Instant::now();
        
        // Build filter from search params
        let mut filter = build_text_search_filter(&params.keywords);
        
        // Add additional filters if provided
        if let Some(filters) = &params.filters {
            if let Some(novel_id) = &filters.novel_id {
                // Convert string ID to ObjectId if possible
                if let Ok(oid) = ObjectId::parse_str(novel_id) {
                    filter.insert("novel_id", oid);
                }
            }
            
            if let Some(tags) = &filters.tags {
                filter.insert("tags", doc! { "$in": tags });
            }
        }
        
        // Set limit for small context window optimization
        let limit = params.limit.unwrap_or(3);
        let options = FindOptions::builder()
            .limit(limit as i64 + 1) // Fetch one extra to check if there are more
            .build();
        
        // Execute query
        let collection = self.db.get_collection::<QA>("qa");
        let mut cursor = collection.find(filter, options).await?;
        
        // Collect results
        let mut qa_entries = Vec::new();
        while let Some(qa) = cursor.try_next().await? {
            qa_entries.push(qa);
        }
        
        // Check if there are more results
        let has_more = qa_entries.len() > limit as usize;
        if has_more {
            qa_entries.pop(); // Remove the extra item
        }
        
        let query_time = start.elapsed();
        Ok(self.format_response(qa_entries, query_time, has_more, Some(limit)).await)
    }

    async fn search_qa_by_regex(&self, regex_pattern: &str) -> Result<Vec<serde_json::Value>> {
        // Build regex filter
        let filter = doc! {
            "$or": [
                { "question": { "$regex": regex_pattern, "$options": "i" } },
                { "answer": { "$regex": regex_pattern, "$options": "i" } }
            ]
        };
        
        // Execute query
        let collection = self.db.get_collection::<QA>("qa");
        let cursor = collection.find(filter, None).await?;
        let qa_entries: Vec<QA> = cursor.try_collect().await?;
        
        // Convert to serde_json::Value
        let json_entries = serde_json::to_value(qa_entries)?;
        if let serde_json::Value::Array(entries) = json_entries {
            Ok(entries)
        } else {
            Ok(vec![])
        }
    }
    
    async fn search_chapters_by_regex(&self, regex_pattern: &str) -> Result<Vec<serde_json::Value>> {
        // Build regex filter
        let filter = doc! {
            "$or": [
                { "title": { "$regex": regex_pattern, "$options": "i" } },
                { "summary": { "$regex": regex_pattern, "$options": "i" } },
                { "key_points": { "$regex": regex_pattern, "$options": "i" } }
            ]
        };
        
        // Execute query with projection to exclude content for token efficiency
        let options = FindOptions::builder()
            .projection(doc! { "content": 0 })
            .build();
            
        let collection = self.db.get_collection::<Chapter>("chapters");
        let cursor = collection.find(filter, options).await?;
        let chapters: Vec<Chapter> = cursor.try_collect().await?;
        
        // Convert to serde_json::Value
        let json_entries = serde_json::to_value(chapters)?;
        if let serde_json::Value::Array(entries) = json_entries {
            Ok(entries)
        } else {
            Ok(vec![])
        }
    }
    
    async fn search_characters_by_regex(&self, regex_pattern: &str) -> Result<Vec<serde_json::Value>> {
        // Build regex filter
        let filter = doc! {
            "$or": [
                { "name": { "$regex": regex_pattern, "$options": "i" } },
                { "description": { "$regex": regex_pattern, "$options": "i" } },
                { "key_traits": { "$regex": regex_pattern, "$options": "i" } }
            ]
        };
        
        // Execute query
        let collection = self.db.get_collection::<Character>("characters");
        let cursor = collection.find(filter, None).await?;
        let characters: Vec<Character> = cursor.try_collect().await?;
        
        // Convert to serde_json::Value
        let json_entries = serde_json::to_value(characters)?;
        if let serde_json::Value::Array(entries) = json_entries {
            Ok(entries)
        } else {
            Ok(vec![])
        }
    }
    
    async fn update_chapter_summary(&self, chapter_id: &str, new_summary: &str) -> Result<()> {
        // Convert string ID to ObjectId
        let object_id = ObjectId::parse_str(chapter_id)?;
        
        // Create update document
        let update = doc! {
            "$set": {
                "summary": new_summary
            }
        };
        
        // Execute update
        let collection = self.db.get_collection::<Chapter>("chapters");
        let result = collection.update_one(doc! { "_id": object_id }, update, None).await?;
        
        if result.matched_count == 0 {
            return Err(anyhow::anyhow!("Chapter not found"));
        }
        
        Ok(())
    }

    async fn search_all(&self, params: &SearchParams) -> Result<serde_json::Value> {
        // Search all collections in parallel for token-efficient results
        let novels_future = self.search_novels(params);
        let chapters_future = self.search_chapters(params);
        let characters_future = self.search_characters(params);
        let qa_future = self.search_qa(params);
        
        // Execute all searches in parallel
        let (novels_result, chapters_result, characters_result, qa_result) = tokio::join!(
            novels_future,
            chapters_future,
            characters_future,
            qa_future
        );
        
        // Extract data from each result
        let novels_data = novels_result?.data;
        let chapters_data = chapters_result?.data;
        let characters_data = characters_result?.data;
        let qa_data = qa_result?.data;
        
        // Create a combined result object
        let combined = serde_json::json!({
            "novels": novels_data,
            "chapters": chapters_data,
            "characters": characters_data,
            "qa": qa_data
        });
        
        Ok(combined)
    }

    async fn get_chapter_content(&self, chapter_id: &str) -> Result<Option<String>> {
        // Convert string ID to ObjectId
        let object_id = match ObjectId::parse_str(chapter_id) {
            Ok(oid) => oid,
            Err(_) => return Ok(None), // Invalid ID format, return None
        };
        
        // Query for the chapter
        let filter = doc! { "_id": object_id };
        let collection = self.db.get_collection::<Chapter>("chapters");
        
        if let Some(chapter) = collection.find_one(filter, None).await? {
            // Return the content if available, otherwise return the summary
            if let Some(content) = chapter.content {
                Ok(Some(content))
            } else if let summary = chapter.summary {
                Ok(Some(format!("Summary: {}", summary)))
            } else {
                Ok(Some("No content or summary available for this chapter.".to_string()))
            }
        } else {
            Ok(None) // Chapter not found
        }
    }

    async fn get_character_details(&self, character_id: &str) -> Result<Option<Character>> {
        // Convert string ID to ObjectId
        let object_id = match ObjectId::parse_str(character_id) {
            Ok(oid) => oid,
            Err(_) => return Ok(None), // Invalid ID format, return None
        };
        
        // Query for the character
        let filter = doc! { "_id": object_id };
        let collection = self.db.get_collection::<Character>("characters");
        
        // Return the character if found
        let character = collection.find_one(filter, None).await?;
        Ok(character)
    }
}