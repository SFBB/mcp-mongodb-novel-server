use anyhow::Result;
use async_trait::async_trait;
use mongodb::{
    bson::{doc, Document, oid::ObjectId},
    options::FindOptions,
};
use std::time::{Duration, Instant};

use crate::db::DatabaseConnection;
use crate::models::{Chapter, Character, MCPResponse, Novel, QA, ResponseMetadata, SearchParams};

#[async_trait]
pub trait DatabaseService {
    async fn search_novels(&self, params: &SearchParams) -> Result<MCPResponse>;
    async fn search_chapters(&self, params: &SearchParams) -> Result<MCPResponse>;
    async fn search_characters(&self, params: &SearchParams) -> Result<MCPResponse>;
    async fn search_qa(&self, params: &SearchParams) -> Result<MCPResponse>;
}

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

    // Helper to convert MongoDB documents to JSON
    async fn format_response<T>(&self, 
        data: Vec<T>, 
        query_time: Duration, 
        has_more: bool,
        limit: Option<u32>
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

#[async_trait]
impl DatabaseService for MongoDBService {
    async fn search_novels(&self, params: &SearchParams) -> Result<MCPResponse> {
        let start = Instant::now();
        
        // Build filter from search params
        let mut filter = Self::build_text_search_filter(&params.keywords);
        
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
        self.format_response(novels, query_time, has_more, Some(limit)).await
    }
    
    async fn search_chapters(&self, params: &SearchParams) -> Result<MCPResponse> {
        let start = Instant::now();
        
        // Build filter from search params
        let mut filter = Self::build_text_search_filter(&params.keywords);
        
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
        self.format_response(chapters, query_time, has_more, Some(limit)).await
    }
    
    async fn search_characters(&self, params: &SearchParams) -> Result<MCPResponse> {
        let start = Instant::now();
        
        // Build filter from search params
        let mut filter = Self::build_text_search_filter(&params.keywords);
        
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
        self.format_response(characters, query_time, has_more, Some(limit)).await
    }
    
    async fn search_qa(&self, params: &SearchParams) -> Result<MCPResponse> {
        let start = Instant::now();
        
        // Build filter from search params
        let mut filter = Self::build_text_search_filter(&params.keywords);
        
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
        self.format_response(qa_entries, query_time, has_more, Some(limit)).await
    }
}