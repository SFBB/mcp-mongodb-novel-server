use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post, delete, patch},
    Router,
};
use mongodb::bson::{doc, oid::ObjectId};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::models::{Novel, Chapter, Character, QA};
use crate::services::{NovelCrudService, ChapterCrudService, CharacterCrudService, QACrudService};
use crate::services::crud_service::CrudService; // Import the CrudService trait

// Novel CRUD handlers
pub async fn get_novels(
    State(novel_service): State<Arc<NovelCrudService>>,
) -> impl IntoResponse {
    match novel_service.read_many(doc! {}, Some(100)).await {
        Ok(novels) => (StatusCode::OK, Json(json!({ "novels": novels }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

pub async fn get_novel(
    State(novel_service): State<Arc<NovelCrudService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match ObjectId::parse_str(&id) {
        Ok(object_id) => match novel_service.read_by_id(&object_id).await {
            Ok(Some(novel)) => (StatusCode::OK, Json(json!({ "novel": novel }))),
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Novel not found" })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            ),
        },
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid ObjectId format" })),
        ),
    }
}

pub async fn create_novel(
    State(novel_service): State<Arc<NovelCrudService>>,
    Json(novel): Json<Novel>,
) -> impl IntoResponse {
    match novel_service.create(&novel).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({ "id": id.to_string(), "message": "Novel created successfully" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

pub async fn update_novel(
    State(novel_service): State<Arc<NovelCrudService>>,
    Path(id): Path<String>,
    Json(update_data): Json<Value>,
) -> impl IntoResponse {
    match ObjectId::parse_str(&id) {
        Ok(object_id) => {
            // Convert serde_json::Value to MongoDB Document
            let bson_doc = match mongodb::bson::to_document(&update_data) {
                Ok(doc) => doc,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": format!("Invalid update data: {}", e) })),
                    )
                }
            };

            match novel_service.update(&object_id, bson_doc).await {
                Ok(Some(novel)) => (
                    StatusCode::OK,
                    Json(json!({ "message": "Novel updated successfully", "novel": novel })),
                ),
                Ok(None) => (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "Novel not found" })),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                ),
            }
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid ObjectId format" })),
        ),
    }
}

pub async fn delete_novel(
    State(novel_service): State<Arc<NovelCrudService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match ObjectId::parse_str(&id) {
        Ok(object_id) => match novel_service.delete(&object_id).await {
            Ok(true) => (
                StatusCode::OK,
                Json(json!({ "message": "Novel deleted successfully" })),
            ),
            Ok(false) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Novel not found" })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            ),
        },
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid ObjectId format" })),
        ),
    }
}

// Chapter CRUD handlers
pub async fn get_chapters(
    State(chapter_service): State<Arc<ChapterCrudService>>,
) -> impl IntoResponse {
    match chapter_service.read_many(doc! {}, Some(100)).await {
        Ok(chapters) => (StatusCode::OK, Json(json!({ "chapters": chapters }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

pub async fn get_novel_chapters(
    State(chapter_service): State<Arc<ChapterCrudService>>,
    Path(novel_id): Path<String>,
) -> impl IntoResponse {
    match ObjectId::parse_str(&novel_id) {
        Ok(object_id) => match chapter_service.find_by_novel_id(&object_id).await {
            Ok(chapters) => (StatusCode::OK, Json(json!({ "chapters": chapters }))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            ),
        },
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid ObjectId format" })),
        ),
    }
}

pub async fn create_chapter(
    State(chapter_service): State<Arc<ChapterCrudService>>,
    Json(chapter): Json<Chapter>,
) -> impl IntoResponse {
    match chapter_service.create(&chapter).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({ "id": id.to_string(), "message": "Chapter created successfully" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

// Character CRUD handlers
pub async fn get_characters(
    State(character_service): State<Arc<CharacterCrudService>>,
) -> impl IntoResponse {
    match character_service.read_many(doc! {}, Some(100)).await {
        Ok(characters) => (StatusCode::OK, Json(json!({ "characters": characters }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

pub async fn get_novel_characters(
    State(character_service): State<Arc<CharacterCrudService>>,
    Path(novel_id): Path<String>,
) -> impl IntoResponse {
    match ObjectId::parse_str(&novel_id) {
        Ok(object_id) => match character_service.find_by_novel_id(&object_id).await {
            Ok(characters) => (StatusCode::OK, Json(json!({ "characters": characters }))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            ),
        },
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid ObjectId format" })),
        ),
    }
}

pub async fn create_character(
    State(character_service): State<Arc<CharacterCrudService>>,
    Json(character): Json<Character>,
) -> impl IntoResponse {
    match character_service.create(&character).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({ "id": id.to_string(), "message": "Character created successfully" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

// QA CRUD handlers
pub async fn get_qa_entries(
    State(qa_service): State<Arc<QACrudService>>,
) -> impl IntoResponse {
    match qa_service.read_many(doc! {}, Some(100)).await {
        Ok(qa_entries) => (StatusCode::OK, Json(json!({ "qa_entries": qa_entries }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

pub async fn create_qa(
    State(qa_service): State<Arc<QACrudService>>,
    Json(qa): Json<QA>,
) -> impl IntoResponse {
    match qa_service.create(&qa).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({ "id": id.to_string(), "message": "QA created successfully" })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

// Create API router
pub fn api_router(
    novel_service: Arc<NovelCrudService>,
    chapter_service: Arc<ChapterCrudService>,
    character_service: Arc<CharacterCrudService>,
    qa_service: Arc<QACrudService>,
) -> Router {
    // Create separate routers for each service with their own state
    let novel_router = Router::new()
        .route("/api/novels", get(get_novels))
        .route("/api/novels", post(create_novel))
        .route("/api/novels/:id", get(get_novel))
        .route("/api/novels/:id", patch(update_novel))
        .route("/api/novels/:id", delete(delete_novel))
        .with_state(novel_service);
        
    let chapter_router = Router::new()
        .route("/api/chapters", get(get_chapters))
        .route("/api/chapters", post(create_chapter))
        .route("/api/novels/:id/chapters", get(get_novel_chapters))
        .with_state(chapter_service);
        
    let character_router = Router::new()
        .route("/api/characters", get(get_characters))
        .route("/api/characters", post(create_character))
        .route("/api/novels/:id/characters", get(get_novel_characters))
        .with_state(character_service);
        
    let qa_router = Router::new()
        .route("/api/qa", get(get_qa_entries))
        .route("/api/qa", post(create_qa))
        .with_state(qa_service);
    
    // Merge all the routers
    Router::new()
        .merge(novel_router)
        .merge(chapter_router)
        .merge(character_router)
        .merge(qa_router)
}