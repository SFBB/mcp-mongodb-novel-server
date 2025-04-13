use anyhow::Result;
use async_trait::async_trait;
use mongodb::{
    bson::{doc, oid::ObjectId, Document},
    options::{FindOneAndUpdateOptions, ReturnDocument},
};
use futures::TryStreamExt; // Add TryStreamExt trait

use crate::db::DatabaseConnection;
use crate::models::{Novel, Chapter, Character, QA};

/// CRUD operations trait for MongoDB collections
#[async_trait]
pub trait CrudService<T> {
    /// Create a new document in the collection
    async fn create(&self, item: &T) -> Result<ObjectId>;
    
    /// Read a document by its ID
    async fn read_by_id(&self, id: &ObjectId) -> Result<Option<T>>;
    
    /// Read multiple documents matching a filter
    async fn read_many(&self, filter: Document, limit: Option<i64>) -> Result<Vec<T>>;
    
    /// Update a document by its ID
    async fn update(&self, id: &ObjectId, update: Document) -> Result<Option<T>>;
    
    /// Delete a document by its ID
    async fn delete(&self, id: &ObjectId) -> Result<bool>;
}

/// MongoDB CRUD implementation for Novel collection
pub struct NovelCrudService {
    db: DatabaseConnection,
}

impl NovelCrudService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    /// Find novels by title (case-insensitive)
    pub async fn find_by_title(&self, title: &str) -> Result<Vec<Novel>> {
        let collection = self.db.get_collection::<Novel>("novels");
        let filter = doc! {
            "title": { "$regex": title, "$options": "i" }
        };
        
        let cursor = collection.find(filter, None).await?;
        let novels: Vec<Novel> = cursor.try_collect().await?;
        
        Ok(novels)
    }
    
    /// Find novels by author (case-insensitive)
    pub async fn find_by_author(&self, author: &str) -> Result<Vec<Novel>> {
        let collection = self.db.get_collection::<Novel>("novels");
        let filter = doc! {
            "author": { "$regex": author, "$options": "i" }
        };
        
        let cursor = collection.find(filter, None).await?;
        let novels: Vec<Novel> = cursor.try_collect().await?;
        
        Ok(novels)
    }
    
    /// Find novels by tags (any match)
    pub async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<Novel>> {
        let collection = self.db.get_collection::<Novel>("novels");
        let filter = doc! {
            "tags": { "$in": tags }
        };
        
        let cursor = collection.find(filter, None).await?;
        let novels: Vec<Novel> = cursor.try_collect().await?;
        
        Ok(novels)
    }
}

#[async_trait]
impl CrudService<Novel> for NovelCrudService {
    async fn create(&self, novel: &Novel) -> Result<ObjectId> {
        let collection = self.db.get_collection::<Novel>("novels");
        
        // Make a clone of the novel since we can't modify the input
        let mut novel_to_insert = novel.clone();
        // Make sure the ID is None to let MongoDB generate one
        novel_to_insert.id = None;
        
        let result = collection.insert_one(novel_to_insert, None).await?;
        
        Ok(result.inserted_id.as_object_id().unwrap())
    }
    
    async fn read_by_id(&self, id: &ObjectId) -> Result<Option<Novel>> {
        let collection = self.db.get_collection::<Novel>("novels");
        
        let filter = doc! { "_id": id };
        let novel = collection.find_one(filter, None).await?;
        
        Ok(novel)
    }
    
    async fn read_many(&self, filter: Document, limit: Option<i64>) -> Result<Vec<Novel>> {
        let collection = self.db.get_collection::<Novel>("novels");
        
        let mut options = None;
        if let Some(limit) = limit {
            options = Some(mongodb::options::FindOptions::builder().limit(limit).build());
        }
        
        let cursor = collection.find(filter, options).await?;
        let novels: Vec<Novel> = cursor.try_collect().await?;
        
        Ok(novels)
    }
    
    async fn update(&self, id: &ObjectId, update: Document) -> Result<Option<Novel>> {
        let collection = self.db.get_collection::<Novel>("novels");
        
        let filter = doc! { "_id": id };
        let update = doc! { "$set": update };
        
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
        
        let novel = collection.find_one_and_update(filter, update, options).await?;
        
        Ok(novel)
    }
    
    async fn delete(&self, id: &ObjectId) -> Result<bool> {
        let collection = self.db.get_collection::<Novel>("novels");
        
        let filter = doc! { "_id": id };
        let result = collection.delete_one(filter, None).await?;
        
        Ok(result.deleted_count > 0)
    }
}

/// MongoDB CRUD implementation for Chapter collection
pub struct ChapterCrudService {
    db: DatabaseConnection,
}

impl ChapterCrudService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    /// Find chapters by novel ID
    pub async fn find_by_novel_id(&self, novel_id: &ObjectId) -> Result<Vec<Chapter>> {
        let collection = self.db.get_collection::<Chapter>("chapters");
        let filter = doc! {
            "novel_id": novel_id
        };
        
        let cursor = collection.find(filter, None).await?;
        let chapters: Vec<Chapter> = cursor.try_collect().await?;
        
        Ok(chapters)
    }
    
    /// Find a specific chapter number in a novel
    pub async fn find_by_novel_and_number(&self, novel_id: &ObjectId, chapter_number: u32) -> Result<Option<Chapter>> {
        let collection = self.db.get_collection::<Chapter>("chapters");
        let filter = doc! {
            "novel_id": novel_id,
            "number": chapter_number
        };
        
        let chapter = collection.find_one(filter, None).await?;
        
        Ok(chapter)
    }
    
    /// Get the full content of a chapter by ID
    pub async fn get_chapter_content(&self, id: &ObjectId) -> Result<Option<String>> {
        let collection = self.db.get_collection::<Chapter>("chapters");
        let filter = doc! { "_id": id };
        let projection = doc! { "content": 1 };
        
        let options = mongodb::options::FindOneOptions::builder()
            .projection(projection)
            .build();
        
        let chapter = collection.find_one(filter, options).await?;
        
        if let Some(chapter) = chapter {
            Ok(chapter.content)
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl CrudService<Chapter> for ChapterCrudService {
    async fn create(&self, chapter: &Chapter) -> Result<ObjectId> {
        let collection = self.db.get_collection::<Chapter>("chapters");
        
        // Make a clone of the chapter since we can't modify the input
        let mut chapter_to_insert = chapter.clone();
        // Make sure the ID is None to let MongoDB generate one
        chapter_to_insert.id = None;
        
        let result = collection.insert_one(chapter_to_insert, None).await?;
        
        Ok(result.inserted_id.as_object_id().unwrap())
    }
    
    async fn read_by_id(&self, id: &ObjectId) -> Result<Option<Chapter>> {
        let collection = self.db.get_collection::<Chapter>("chapters");
        
        let filter = doc! { "_id": id };
        let chapter = collection.find_one(filter, None).await?;
        
        Ok(chapter)
    }
    
    async fn read_many(&self, filter: Document, limit: Option<i64>) -> Result<Vec<Chapter>> {
        let collection = self.db.get_collection::<Chapter>("chapters");
        
        let mut options = None;
        if let Some(limit) = limit {
            options = Some(mongodb::options::FindOptions::builder().limit(limit).build());
        }
        
        let cursor = collection.find(filter, options).await?;
        let chapters: Vec<Chapter> = cursor.try_collect().await?;
        
        Ok(chapters)
    }
    
    async fn update(&self, id: &ObjectId, update: Document) -> Result<Option<Chapter>> {
        let collection = self.db.get_collection::<Chapter>("chapters");
        
        let filter = doc! { "_id": id };
        let update = doc! { "$set": update };
        
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
        
        let chapter = collection.find_one_and_update(filter, update, options).await?;
        
        Ok(chapter)
    }
    
    async fn delete(&self, id: &ObjectId) -> Result<bool> {
        let collection = self.db.get_collection::<Chapter>("chapters");
        
        let filter = doc! { "_id": id };
        let result = collection.delete_one(filter, None).await?;
        
        Ok(result.deleted_count > 0)
    }
}

/// MongoDB CRUD implementation for Character collection
pub struct CharacterCrudService {
    db: DatabaseConnection,
}

impl CharacterCrudService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    /// Find characters by novel ID
    pub async fn find_by_novel_id(&self, novel_id: &ObjectId) -> Result<Vec<Character>> {
        let collection = self.db.get_collection::<Character>("characters");
        let filter = doc! {
            "novel_id": novel_id
        };
        
        let cursor = collection.find(filter, None).await?;
        let characters: Vec<Character> = cursor.try_collect().await?;
        
        Ok(characters)
    }
    
    /// Find character by name in a specific novel (case-insensitive)
    pub async fn find_by_novel_and_name(&self, novel_id: &ObjectId, name: &str) -> Result<Option<Character>> {
        let collection = self.db.get_collection::<Character>("characters");
        let filter = doc! {
            "novel_id": novel_id,
            "name": { "$regex": name, "$options": "i" }
        };
        
        let character = collection.find_one(filter, None).await?;
        
        Ok(character)
    }
    
    /// Find characters by role in a specific novel
    pub async fn find_by_novel_and_role(&self, novel_id: &ObjectId, role: &str) -> Result<Vec<Character>> {
        let collection = self.db.get_collection::<Character>("characters");
        let filter = doc! {
            "novel_id": novel_id,
            "role": role
        };
        
        let cursor = collection.find(filter, None).await?;
        let characters: Vec<Character> = cursor.try_collect().await?;
        
        Ok(characters)
    }
}

#[async_trait]
impl CrudService<Character> for CharacterCrudService {
    async fn create(&self, character: &Character) -> Result<ObjectId> {
        let collection = self.db.get_collection::<Character>("characters");
        
        // Make a clone of the character since we can't modify the input
        let mut character_to_insert = character.clone();
        // Make sure the ID is None to let MongoDB generate one
        character_to_insert.id = None;
        
        let result = collection.insert_one(character_to_insert, None).await?;
        
        Ok(result.inserted_id.as_object_id().unwrap())
    }
    
    async fn read_by_id(&self, id: &ObjectId) -> Result<Option<Character>> {
        let collection = self.db.get_collection::<Character>("characters");
        
        let filter = doc! { "_id": id };
        let character = collection.find_one(filter, None).await?;
        
        Ok(character)
    }
    
    async fn read_many(&self, filter: Document, limit: Option<i64>) -> Result<Vec<Character>> {
        let collection = self.db.get_collection::<Character>("characters");
        
        let mut options = None;
        if let Some(limit) = limit {
            options = Some(mongodb::options::FindOptions::builder().limit(limit).build());
        }
        
        let cursor = collection.find(filter, options).await?;
        let characters: Vec<Character> = cursor.try_collect().await?;
        
        Ok(characters)
    }
    
    async fn update(&self, id: &ObjectId, update: Document) -> Result<Option<Character>> {
        let collection = self.db.get_collection::<Character>("characters");
        
        let filter = doc! { "_id": id };
        let update = doc! { "$set": update };
        
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
        
        let character = collection.find_one_and_update(filter, update, options).await?;
        
        Ok(character)
    }
    
    async fn delete(&self, id: &ObjectId) -> Result<bool> {
        let collection = self.db.get_collection::<Character>("characters");
        
        let filter = doc! { "_id": id };
        let result = collection.delete_one(filter, None).await?;
        
        Ok(result.deleted_count > 0)
    }
}

/// MongoDB CRUD implementation for QA collection
pub struct QACrudService {
    db: DatabaseConnection,
}

impl QACrudService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    /// Find Q&A entries by novel ID
    pub async fn find_by_novel_id(&self, novel_id: &ObjectId) -> Result<Vec<QA>> {
        let collection = self.db.get_collection::<QA>("qa");
        let filter = doc! {
            "novel_id": novel_id
        };
        
        let cursor = collection.find(filter, None).await?;
        let qa_entries: Vec<QA> = cursor.try_collect().await?;
        
        Ok(qa_entries)
    }
    
    /// Find Q&A entries by tags
    pub async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<QA>> {
        let collection = self.db.get_collection::<QA>("qa");
        let filter = doc! {
            "tags": { "$in": tags }
        };
        
        let cursor = collection.find(filter, None).await?;
        let qa_entries: Vec<QA> = cursor.try_collect().await?;
        
        Ok(qa_entries)
    }
    
    /// Text search in questions and answers
    pub async fn search_text(&self, query: &str) -> Result<Vec<QA>> {
        let collection = self.db.get_collection::<QA>("qa");
        let filter = doc! {
            "$text": {
                "$search": query
            }
        };
        
        let cursor = collection.find(filter, None).await?;
        let qa_entries: Vec<QA> = cursor.try_collect().await?;
        
        Ok(qa_entries)
    }
}

#[async_trait]
impl CrudService<QA> for QACrudService {
    async fn create(&self, qa: &QA) -> Result<ObjectId> {
        let collection = self.db.get_collection::<QA>("qa");
        
        // Make a clone of the QA since we can't modify the input
        let mut qa_to_insert = qa.clone();
        // Make sure the ID is None to let MongoDB generate one
        qa_to_insert.id = None;
        
        let result = collection.insert_one(qa_to_insert, None).await?;
        
        Ok(result.inserted_id.as_object_id().unwrap())
    }
    
    async fn read_by_id(&self, id: &ObjectId) -> Result<Option<QA>> {
        let collection = self.db.get_collection::<QA>("qa");
        
        let filter = doc! { "_id": id };
        let qa = collection.find_one(filter, None).await?;
        
        Ok(qa)
    }
    
    async fn read_many(&self, filter: Document, limit: Option<i64>) -> Result<Vec<QA>> {
        let collection = self.db.get_collection::<QA>("qa");
        
        let mut options = None;
        if let Some(limit) = limit {
            options = Some(mongodb::options::FindOptions::builder().limit(limit).build());
        }
        
        let cursor = collection.find(filter, options).await?;
        let qa_entries: Vec<QA> = cursor.try_collect().await?;
        
        Ok(qa_entries)
    }
    
    async fn update(&self, id: &ObjectId, update: Document) -> Result<Option<QA>> {
        let collection = self.db.get_collection::<QA>("qa");
        
        let filter = doc! { "_id": id };
        let update = doc! { "$set": update };
        
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
        
        let qa = collection.find_one_and_update(filter, update, options).await?;
        
        Ok(qa)
    }
    
    async fn delete(&self, id: &ObjectId) -> Result<bool> {
        let collection = self.db.get_collection::<QA>("qa");
        
        let filter = doc! { "_id": id };
        let result = collection.delete_one(filter, None).await?;
        
        Ok(result.deleted_count > 0)
    }
}