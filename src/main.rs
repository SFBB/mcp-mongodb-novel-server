use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::post,
    Router,
};
use dotenv::dotenv;

mod db;
mod handlers;
mod mcp;
mod models;
mod services;
mod utils;

use handlers::{mcp_handler, ServerState, api_router};
use services::{MongoDBService, NovelCrudService, ChapterCrudService, CharacterCrudService, QACrudService};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();
    
    // Get port from environment or use default
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");
    
    // Create database connection
    let db_connection = db::DatabaseConnection::new().await?;
    
    // Create database service for MCP
    let db_service = MongoDBService::new().await?;
    
    // Create CRUD services
    let novel_service = Arc::new(NovelCrudService::new(db_connection.clone()));
    let chapter_service = Arc::new(ChapterCrudService::new(db_connection.clone()));
    let character_service = Arc::new(CharacterCrudService::new(db_connection.clone()));
    let qa_service = Arc::new(QACrudService::new(db_connection.clone()));
    
    // Set up MCP application state
    let mcp_state = Arc::new(ServerState {
        db_service,
    });
    
    // Build MCP endpoint
    let mcp_app = Router::new()
        .route("/mcp", post(mcp_handler::<MongoDBService>))
        .with_state(mcp_state);
    
    // Build CRUD API router
    let api_app = api_router(
        novel_service,
        chapter_service,
        character_service,
        qa_service,
    );
    
    // Merge routers
    let app = Router::new()
        .merge(mcp_app)
        .merge(api_app);
    
    // Run it
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on http://{}", addr);
    tracing::info!("MCP endpoint available at http://{}:{}/mcp", addr.ip(), port);
    tracing::info!("CRUD API endpoints available at http://{}:{}/api/...", addr.ip(), port);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
