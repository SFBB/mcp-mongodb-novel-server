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
- **Python Scrapers**: Integrated scrapers for populating the database from various sources

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
- **Scraper Integration**: Python scrapers to help populate the database

## Getting Started

### Prerequisites

- Rust (latest stable version)
- MongoDB instance with your domain data
- Python 3.7+ (for scrapers)

### Installation

1. Clone this repository with submodules
   ```
   git clone --recursive https://github.com/YOUR-USERNAME/mcp-mongodb-server.git
   ```
   
   If you've already cloned without `--recursive`, you can get the submodules with:
   ```
   git submodule update --init --recursive
   ```

2. Create a `.env` file with the following variables:
   ```
   MONGODB_URI=mongodb://localhost:27017
   DATABASE_NAME=your_database_name
   PORT=3000
   ```
3. Build and run the MCP server:
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

For detailed schema information and indexing recommendations, see the [Database Interface Documentation](docs/database_interface.md).

## Usage

### MCP Endpoint

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

### CRUD API

The server also provides REST API endpoints for managing database content:

- `GET /api/novels` - List all novels
- `POST /api/novels` - Create a new novel
- `GET /api/novels/:id` - Get novel details
- `PATCH /api/novels/:id` - Update a novel
- `DELETE /api/novels/:id` - Delete a novel

Similar endpoints exist for chapters, characters, and Q&A entries.

### Python Scrapers

The project includes Python scrapers (as a Git submodule) for populating the database:

1. Set up the Python environment:
   ```
   cd scraper_library
   pip install -r requirements.txt
   ```

2. Use the scrapers to populate your database:
   ```
   python -m src.scrape_<source> --help
   ```

Available scrapers include:
- `scrape_69shunet.py` - 69Shu.net
- `scrape_baobao88.py` - BaoBao88
- `scrape_quanben.py` - Quanben
- `scrape_syosetu.py` - Syosetu
- `scrape_ximalaya.py` - Ximalaya

Check each scraper's documentation for specific usage instructions.

### Example Queries

- "What happens in chapter 3 of the novel?"
- "Tell me about the protagonist character"
- "Find all Q&A related to magic systems"
- "Summarize the novel's plot"

## License

MIT