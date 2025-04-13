# MCP MongoDB Server

A Model Context Protocol (MCP) server that provides an interface between LLMs and MongoDB databases, optimized for small context windows (3k tokens).

## Overview

This server acts as an assistant for LLMs to get better knowledge of specific domains stored in MongoDB. The data structure and schema are optimized to allow efficient interaction with small context windows.

Example use cases:
- Querying novel chapter information
- Getting character details 
- Accessing author Q&A
- Retrieving domain-specific knowledge

## Features

- **MCP Protocol Implementation**: Follows the Model Context Protocol specification
- **Natural Language Query Parsing**: Convert natural language queries to structured database operations
- **MongoDB Integration**: Query existing MongoDB collections with optimized results
- **Context-Optimized Responses**: Formatted responses designed for small context windows (3k tokens)
- **Domain-Specific Formatting**: Custom formatting for different entity types (novels, chapters, characters, Q&A)

## Data Models

The server uses the following optimized data models:

- **Novels**: Core metadata about novels
- **Chapters**: Summaries and key points (with full content available on demand)
- **Characters**: Character details with compact relationship representations
- **Q&A**: Knowledge base entries with tags for efficient querying

## Architecture

- **MCP Protocol Layer**: Handles JSON-RPC requests/responses following MCP specification
- **Query Parser**: Converts natural language to structured queries
- **Database Service**: Performs optimized MongoDB operations
- **Response Formatter**: Formats responses in an LLM-friendly way

## Getting Started

### Prerequisites

- Rust (latest stable version)
- MongoDB instance with your domain data

### Installation

1. Clone this repository
2. Create a `.env` file with the following variables:
   ```
   MONGODB_URI=mongodb://localhost:27017
   DATABASE_NAME=your_database_name
   PORT=3000
   ```
3. Build and run:
   ```
   cargo build --release
   cargo run --release
   ```

### MongoDB Setup

Your MongoDB should have collections structured according to the models defined in this project:

- `novels`: Novel metadata
- `chapters`: Chapter information
- `characters`: Character details
- `qa`: Question and answer pairs

## Usage

Send MCP-compliant JSON-RPC requests to the `/mcp` endpoint:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "query",
  "params": {
    "query": "Tell me about the main character in the novel"
  }
}
```

### Example Queries

- "What happens in chapter 3 of the novel?"
- "Tell me about the protagonist character"
- "Find all Q&A related to magic systems"
- "Summarize the novel's plot"

## License

MIT