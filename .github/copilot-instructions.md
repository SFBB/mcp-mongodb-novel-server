<!-- Use this file to provide workspace-specific custom instructions to Copilot. For more details, visit https://code.visualstudio.com/docs/copilot/copilot-customization#_use-a-githubcopilotinstructionsmd-file -->

# MCP MongoDB Server - Coding Guidelines

This project is a Model Context Protocol (MCP) server that connects to MongoDB and is optimized for small context windows (3k tokens). The server is designed to help LLMs access specific domain knowledge from MongoDB databases.

## Project Structure
- `src/db/`: Database connection and operations
- `src/models/`: Data models and structures
- `src/services/`: Business logic and database service implementations
- `src/handlers/`: HTTP request handlers
- `src/mcp/`: MCP protocol implementation
- `src/utils/`: Utility functions

## Optimization Guidelines
When working on this codebase, focus on optimizing for small context windows:

1. **Token Efficiency**: Keep responses compact and information-dense
2. **Chunking**: Implement pagination and chunking for large datasets
3. **Summarization**: Prefer summaries and key points over full content
4. **Smart Formatting**: Format responses to maximize information per token
5. **Context Tracking**: Track token usage in responses

## MCP Protocol Notes
Follow the Model Context Protocol specification when implementing new features:
- Use proper JSON-RPC 2.0 formatting
- Include appropriate error codes and messages
- Handle context properly in queries

## Database Design
The MongoDB collections are optimized for:
- Fast retrieval of summarized content
- Relationship navigation with minimal queries
- Efficient text search
- Hierarchical data with cross-references

You can find more info about MCP at https://modelcontextprotocol.io/