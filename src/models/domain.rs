use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;

// Novel metadata - compact representation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Novel {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub title: String,
    pub author: String,
    pub summary: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<NovelMetadata>,
}

// Extended metadata separated to keep main queries light
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NovelMetadata {
    pub publication_date: Option<String>,
    pub genre: Vec<String>,
    pub word_count: Option<u32>,
    pub language: Option<String>,
}

// Chapters - optimized structure with summary and key points
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chapter {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub novel_id: ObjectId,
    pub number: u32,
    pub title: String,
    pub summary: String,
    pub key_points: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>, // Full content stored separately
}

// Characters - focus on key attributes and relationships
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Character {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub novel_id: ObjectId,
    pub name: String,
    pub role: String, // protagonist, antagonist, supporting
    pub description: String,
    pub key_traits: Vec<String>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Relationship {
    pub character_id: Option<ObjectId>,
    pub character_name: String, // Denormalized for efficiency
    pub relationship_type: String, // friend, enemy, family, etc.
}

// Q&A - knowledge base entries
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QA {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub novel_id: Option<ObjectId>,
    pub question: String,
    pub answer: String,
    pub tags: Vec<String>,
}

// Search query parameters - used for MCP requests
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchParams {
    pub collection: String,
    pub query_type: String,
    pub keywords: Vec<String>,
    pub filters: Option<SearchFilters>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchFilters {
    pub novel_id: Option<String>,
    pub character_name: Option<String>,
    pub tags: Option<Vec<String>>,
}

// MCP response - optimized for small context windows
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPResponse {
    pub status: String,
    pub data: serde_json::Value,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseMetadata {
    pub token_count: Option<u32>,
    pub query_time_ms: u64,
    pub has_more: bool,
    pub next_page_token: Option<String>,
}