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
- Git (for cloning and working with the repository)

### Repository Structure

The project is organized as follows:

```
mcp_database/
├── src/              # Rust source code for the MCP server
│   ├── db/           # Database connection and operations
│   ├── handlers/     # HTTP request handlers 
│   ├── mcp/          # MCP protocol implementation
│   ├── models/       # Data models and structures
│   ├── services/     # Business logic implementations
│   └── utils/        # Utility functions
├── docs/             # Documentation
│   └── database_interface.md  # Detailed API and schema docs
└── scraper_library/  # Python scrapers for data collection (submodule)
    └── src/          # Scraper source code for various sites
```

### Git Repository

The project is hosted on GitHub. To contribute or use:

1. **Clone the Repository**:
   ```bash
   # Clone with submodules (recommended)
   git clone --recursive https://github.com/SFBB/mcp-mongodb-novel-server.git
   
   # Or clone normally and then initialize submodules
   git clone https://github.com/SFBB/mcp-mongodb-novel-server.git
   cd mcp-mongodb-novel-server
   git submodule update --init --recursive
   ```

2. **Stay Updated**:
   ```bash
   # Pull latest changes including submodule updates
   git pull --recurse-submodules
   
   # Update only submodules to their latest versions
   git submodule update --remote
   ```

3. **Contributing**:
   ```bash
   # Create a new branch for your feature
   git checkout -b feature/your-feature-name
   
   # Make changes, then commit
   git add .
   git commit -m "Add feature: description of your changes"
   
   # Push your branch
   git push -u origin feature/your-feature-name
   
   # Then create a Pull Request on GitHub
   ```

### Installation

1. Clone this repository with submodules as shown above

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

## Development

### Updating the Scraper Library

The scraper library is included as a Git submodule. To update it to the latest version:

```bash
# Navigate to the submodule directory
cd scraper_library

# Fetch the latest changes
git fetch origin
git checkout main
git pull

# Go back to the main project and commit the submodule update
cd ..
git add scraper_library
git commit -m "Update scraper library to latest version"
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific tests
cargo test db_service
```

### Code Style

This project follows Rust's standard code style guidelines:

```bash
# Check code formatting
cargo fmt -- --check

# Run linting
cargo clippy
```

## Versioning

We use [SemVer](http://semver.org/) for versioning. For the versions available, see the [tags on this repository](https://github.com/SFBB/mcp-mongodb-novel-server/tags).

## License

MIT