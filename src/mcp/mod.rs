pub mod conversion;
pub mod protocol;
pub mod server;

// Re-export key components for easier imports
pub use protocol::{MCPError, MCPParams, MCPRequest, MCPResponse, MCPResult};