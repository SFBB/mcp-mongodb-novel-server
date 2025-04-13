use crate::models::{SearchFilters, SearchParams};
use regex::Regex;
use std::collections::HashSet;

pub struct QueryParser;

impl QueryParser {
    pub fn parse_natural_language_query(query: &str) -> SearchParams {
        // Default to novel collection if not specified
        let collection = Self::extract_collection(query).unwrap_or_else(|| "novels".to_string());
        
        // Extract query type (search, summary, details)
        let query_type = Self::extract_query_type(query).unwrap_or_else(|| "search".to_string());
        
        // Extract keywords for search
        let keywords = Self::extract_keywords(query);
        
        // Extract filters
        let filters = Self::extract_filters(query);
        
        // Default limit
        let limit = Self::extract_limit(query);
        
        SearchParams {
            collection,
            query_type,
            keywords,
            filters: Some(filters),
            limit,
        }
    }
    
    fn extract_collection(query: &str) -> Option<String> {
        let collections = [
            ("novel", "novels"),
            ("chapter", "chapters"),
            ("character", "characters"),
            ("qa", "qa"),
            ("question", "qa"),
            ("answer", "qa"),
        ];
        
        for (keyword, collection) in collections.iter() {
            if query.to_lowercase().contains(keyword) {
                return Some(collection.to_string());
            }
        }
        
        None
    }
    
    fn extract_query_type(query: &str) -> Option<String> {
        let query_types = [
            ("summary", "summary"),
            ("overview", "summary"),
            ("detail", "details"),
            ("information", "details"),
            ("search", "search"),
            ("find", "search"),
            ("list", "list"),
            ("all", "list"),
        ];
        
        for (keyword, query_type) in query_types.iter() {
            if query.to_lowercase().contains(keyword) {
                return Some(query_type.to_string());
            }
        }
        
        None
    }
    
    fn extract_keywords(query: &str) -> Vec<String> {
        // Common stop words to filter out
        let stop_words: HashSet<&str> = [
            "the", "a", "an", "in", "on", "at", "of", "to", "for", "with", "about", "by",
            "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
            "do", "does", "did", "can", "could", "will", "would", "should", "shall",
            "get", "find", "show", "tell", "give", "search", "query", "look",
        ].iter().cloned().collect();
        
        query
            .split_whitespace()
            .map(|word| word.to_lowercase())
            .filter(|word| {
                let word = word.trim_matches(|c: char| !c.is_alphanumeric());
                !stop_words.contains(word) && word.len() > 2
            })
            .collect()
    }
    
    fn extract_filters(query: &str) -> SearchFilters {
        let mut filters = SearchFilters {
            novel_id: None,
            character_name: None,
            tags: None,
        };
        
        // Extract novel ID or name
        if let Some(novel_id) = Self::extract_novel_id(query) {
            filters.novel_id = Some(novel_id);
        }
        
        // Extract character name
        if let Some(character_name) = Self::extract_character_name(query) {
            filters.character_name = Some(character_name);
        }
        
        // Extract tags
        let tags = Self::extract_tags(query);
        if !tags.is_empty() {
            filters.tags = Some(tags);
        }
        
        filters
    }
    
    fn extract_novel_id(query: &str) -> Option<String> {
        // Look for novel ID patterns (might be expanded based on actual ID format)
        let novel_regex = Regex::new(r"novel\s+(?:id|ID)?\s*[:|=]?\s*([a-zA-Z0-9]+)").ok()?;
        novel_regex.captures(query).map(|caps| caps[1].to_string())
    }
    
    fn extract_character_name(query: &str) -> Option<String> {
        // Look for character mentions
        let char_regex = Regex::new(r"character\s+(?:named|called)?\s*[:|=]?\s*([a-zA-Z\s]+)").ok()?;
        char_regex.captures(query).map(|caps| caps[1].trim().to_string())
    }
    
    fn extract_tags(query: &str) -> Vec<String> {
        // Look for tags or categories
        let tag_regex = Regex::new(r"tags?\s*[:|=]?\s*([a-zA-Z,\s]+)").ok();
        
        if let Some(regex) = tag_regex {
            if let Some(caps) = regex.captures(query) {
                return caps[1]
                    .split(',')
                    .map(|tag| tag.trim().to_string())
                    .filter(|tag| !tag.is_empty())
                    .collect();
            }
        }
        
        Vec::new()
    }
    
    fn extract_limit(query: &str) -> Option<u32> {
        // Look for limit specifications
        let limit_regex = Regex::new(r"limit\s*[:|=]?\s*(\d+)").ok()?;
        limit_regex
            .captures(query)
            .and_then(|caps| caps[1].parse::<u32>().ok())
    }
}