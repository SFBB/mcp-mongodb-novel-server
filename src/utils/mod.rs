pub mod query_parser;
pub use query_parser::QueryParser;

use std::collections::HashMap;

/// Validates the provided authentication token.
/// Returns true if the token is valid, false otherwise.
pub fn validate_auth_token(options: &HashMap<String, serde_json::Value>) -> bool {
    if let Some(token) = options.get("auth_token").and_then(|v| v.as_str()) {
        // Replace this with actual token validation logic, such as checking against a database or environment variable.
        const TRUSTED_TOKENS: [&str; 1] = ["trusted_llm_token"];
        TRUSTED_TOKENS.contains(&token)
    } else {
        false
    }
}

/// Generates an error message for unauthorized access.
pub fn unauthorized_error() -> crate::handlers::MCPErrorResponse {
    crate::handlers::MCPErrorResponse {
        code: -32604, // Unauthorized
        message: "Invalid or missing authentication token".to_string(),
    }
}