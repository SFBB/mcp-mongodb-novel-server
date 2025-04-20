use std::net::SocketAddr;
use std::sync::Arc;

use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use axum::{
    routing::post,
    Router,
};
use dotenv::dotenv;
use tokio::net::TcpListener;

use crate::handlers::mcp_handler::sse_handler;

mod db;
mod handlers;
mod mcp;
mod models;
mod services;
mod utils;

use crate::handlers::{
    api_router, 
    mcp_handler::{MpcHandler, ServerState}
};
use crate::services::{
    crud_service::{ChapterCrudService, CharacterCrudService, NovelCrudService, QACrudService},
    db_service::{MongoDBService, DatabaseService},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();
    
    // Check if we should run in stdio mode
    let stdio_mode = std::env::var("STDIO_MODE").unwrap_or_default() == "1";
    
    // Create database service for MCP
    let db_service = Arc::new(MongoDBService::new().await?);
    
    // if stdio_mode {
    //     // Run in stdio mode for direct MCP communication
    //     tracing::info!("Starting MCP server in stdio mode");
    //     let server_state = ServerState {
    //         db_service: db_service.clone(),
    //     };
    //     return run_stdio_mcp_server(server_state).await.map_err(|e| anyhow::anyhow!(e.to_string()));
    // }
    
    // Get port from environment or use default
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");
    
    // Create database connection
    let db_connection = db::DatabaseConnection::new().await?;
    
    // Create CRUD services
    let novel_service = Arc::new(NovelCrudService::new(db_connection.clone()));
    let chapter_service = Arc::new(ChapterCrudService::new(db_connection.clone()));
    let character_service = Arc::new(CharacterCrudService::new(db_connection.clone()));
    let qa_service = Arc::new(QACrudService::new(db_connection.clone()));
    
    // Set up MCP application state
    let mcp_state = ServerState {
        db_service: db_service.clone(),
    };
    
    let mcp_handler = MpcHandler::new(db_service.clone());
    // Build MCP endpoint
    // let mcp_app = Router::new()
    //     .route("/mcp", post(mpc_handler))
    //     .with_state(mcp_state);
    let config = SseServerConfig {
        bind: SocketAddr::from(([0, 0, 0, 0], port)),
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: tokio_util::sync::CancellationToken::new(),
    };
    let sse_server = SseServer::serve_with_config(config).await?;
    sse_server.with_service(move || MpcHandler::new(db_service.clone()));
    
    // Build CRUD API router
    let api_app = api_router(
        novel_service,
        chapter_service,
        character_service,
        qa_service,
    );
    
    // Merge routers
    let app = Router::new()
        .merge(api_app);
    
    // Run it
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on http://{}", addr);
    tracing::info!("MCP endpoint available at http://{}:{}/mcp", addr.ip(), port);
    tracing::info!("CRUD API endpoints available at http://{}:{}/api/...", addr.ip(), port);
    
    // Create a TCP listener
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
