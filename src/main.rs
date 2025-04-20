use std::net::SocketAddr;
use std::sync::Arc;

use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use axum::{
    routing::post,
    Router,
};
use dotenv::dotenv;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

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
    
    // Get base port from environment or use default
    let base_port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let sse_port = base_port;
    let api_port = base_port + 1; // Assign a different port for CRUD API

    // Create database service (shared)
    let db_service: Arc<MongoDBService> = Arc::new(MongoDBService::new().await?);
    let db_connection = db_service.get_db_connection(); // Assuming a method to get the raw connection for CRUD

    // --- Start SSE Server ---
    let sse_addr = SocketAddr::from(([0, 0, 0, 0], sse_port));
    let sse_cancellation_token = CancellationToken::new();
    let config = SseServerConfig {
        bind: sse_addr,
        sse_path: "/sse".to_string(), // Standard SSE path
        post_path: "/message".to_string(), // Standard POST path for rmcp SSE
        ct: sse_cancellation_token.clone(),
    };
    // Use the db_service clone for the SSE handler
    let sse_server = SseServer::serve_with_config(config).await?;
    // Start the SSE server by attaching the service; it runs in the background
    let ct = sse_server.with_service(move || MpcHandler::new(db_service.clone()));
    tracing::info!("SSE Server listening on http://{}", sse_addr);
    tracing::info!("MCP SSE endpoint available at http://{}:{}/sse", sse_addr.ip(), sse_port);
    tracing::info!("MCP POST endpoint available at http://{}:{}/message", sse_addr.ip(), sse_port);


    // --- Start CRUD API Server ---
    // Create CRUD services using the db_connection
    let novel_service = Arc::new(NovelCrudService::new(db_connection.clone()));
    let chapter_service = Arc::new(ChapterCrudService::new(db_connection.clone()));
    let character_service = Arc::new(CharacterCrudService::new(db_connection.clone()));
    let qa_service = Arc::new(QACrudService::new(db_connection.clone()));

    // Build CRUD API router
    let api_app = api_router(
        novel_service,
        chapter_service,
        character_service,
        qa_service,
    );
    let app = Router::new().merge(api_app); // Only includes /api routes

    // Run CRUD API server on its own port
    let api_addr = SocketAddr::from(([0, 0, 0, 0], api_port));
    tracing::info!("CRUD API Server listening on http://{}", api_addr);
    tracing::info!("CRUD API endpoints available at http://{}:{}/api/...", api_addr.ip(), api_port);

    let listener = TcpListener::bind(api_addr).await?;
    // Spawn the Axum server task
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("CRUD API server failed");
    });

    // Wait for Ctrl+C signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received...");

    // Cancel the SseServer tasks
    ct.cancel();
    // Optionally add graceful shutdown for the Axum server if needed

    tracing::info!("Servers shut down gracefully.");
    Ok(())
}

// Helper extension trait to get DB connection (example)
trait DbServiceExt {
    fn get_db_connection(&self) -> crate::db::DatabaseConnection;
}

impl DbServiceExt for MongoDBService {
    fn get_db_connection(&self) -> crate::db::DatabaseConnection {
        // Assuming MongoDBService has a field `db` of type DatabaseConnection
        // Adjust this based on your actual MongoDBService implementation
        self.db.clone()
    }
}
