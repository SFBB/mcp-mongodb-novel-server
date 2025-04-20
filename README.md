# MCP MongoDB Server

A high-performance [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server that provides an efficient knowledge interface between Large Language Models (LLMs) and MongoDB. Optimized for small context windows (3k tokens), this server enables LLMs to retrieve and interact with domain-specific knowledge stored in MongoDB collections.

## 🚀 Features

- **Dual-Server Architecture**:
  - SSE-based MCP server for efficient LLM communication
  - RESTful CRUD API for database management
  
- **Token Efficiency**: Responses formatted for maximum information density within small context windows

- **MongoDB Integration**: Works with existing MongoDB collections with optimized query mechanisms

- **Data Models** for multiple domain-specific entities:
  - Novels
  - Chapters
  - Characters
  - Q&A Knowledge Base

- **Python Scraper Integrations**: Included as submodules for populating databases from various sources

## 🔧 Architecture

```
                    ┌───────────────────┐
                    │                   │
                    │     Language      │
                    │      Models       │
                    │                   │
                    └─────────┬─────────┘
                              │
                              │ MCP (JSON-RPC 2.0)
                              ▼
┌─────────────────────────────────────────────────────┐
│                                                     │
│  ┌─────────────────┐           ┌─────────────────┐  │
│  │                 │           │                 │  │
│  │   SSE Server    │◄────────►│   MCP Handler   │  │
│  │  (Port 3000)    │           │                 │  │
│  └─────────────────┘           └────────┬────────┘  │
│                                         │           │
│                                         ▼           │
│                                ┌────────────────┐   │
│  ┌─────────────────┐           │                │   │
│  │                 │           │  DB Services   │   │
│  │   CRUD API      │◄────────►│                │   │
│  │  (Port 3001)    │           └────────────────┘   │
│  └─────────────────┘                   │           │
│                                         │           │
└─────────────────────────────────────────┼───────────┘
                                          │
                                          ▼
                                 ┌─────────────────┐
                                 │                 │
                                 │    MongoDB      │
                                 │                 │
                                 └─────────────────┘
```

## 🏁 Getting Started

### Prerequisites

- Rust (latest stable toolchain)
- MongoDB instance
- Python 3.7+ (for data scrapers)

### Installation

1. **Clone the repository with submodules**:
   ```bash
   git clone --recursive https://github.com/SFBB/mcp-mongodb-novel-server.git
   cd mcp-mongodb-novel-server
   ```

2. **Configure environment**:
   Create a `.env` file in the project root:
   ```
   MONGODB_URI=mongodb://localhost:27017
   DATABASE_NAME=your_database
   PORT=3000  # Base port (MCP SSE server)
               # CRUD API will use PORT+1 (3001)
   ```

3. **Build and run**:
   ```bash
   cargo build --release
   cargo run --release
   ```

Upon successful startup, you'll see:
- MCP SSE endpoint available at http://127.0.0.1:3000/sse
- MCP POST endpoint available at http://127.0.0.1:3000/message
- CRUD API endpoints available at http://127.0.0.1:3001/api/...

### Usage

You firstly need to run this server. Then configure your client based on JSON-RPC or SSE.

This is an exmaple setup for VSCode copilot client.

```JSON
"mcp-mongodb-novel-server": {
    "type": "sse",
    "url": "http://localhost:3000/sse"
}  
```

## 🔌 Server Endpoints

### 1. MCP Server (SSE Protocol)

The MCP server provides two endpoints:
- **SSE Endpoint**: `http://localhost:3000/sse`
  - For establishing SSE connections to receive events
- **POST Endpoint**: `http://localhost:3000/message`
  - For sending JSON-RPC 2.0 formatted commands

Example MCP request (using POST to `/message`):
```json
{
  "jsonrpc": "2.0",
  "id": "request-1",
  "method": "query_character",
  "params": {
    "character_id": "5f8e4c3b2a1d"
  }
}
```

### 2. CRUD API (REST)

The CRUD API provides RESTful endpoints for managing database content:

- **Novels**: `/api/novels`
- **Chapters**: `/api/chapters`
- **Characters**: `/api/characters`
- **Q&A**: `/api/qa`

Standard REST operations (`GET`, `POST`, `PATCH`, `DELETE`) are supported.

## 📊 Data Models

### Novels
```json
{
  "_id": "ObjectId",
  "title": "String",
  "author": "String",
  "summary": "String",
  "year": "Number",
  "tags": ["String"]
}
```

### Chapters
```json
{
  "_id": "ObjectId",
  "novel_id": "ObjectId",
  "title": "String",
  "chapter_number": "Number",
  "summary": "String",
  "content": "String",
  "word_count": "Number"
}
```

### Characters
```json
{
  "_id": "ObjectId",
  "novel_id": "ObjectId",
  "name": "String",
  "description": "String",
  "traits": ["String"],
  "relationships": [
    {
      "character_id": "ObjectId",
      "type": "String"
    }
  ]
}
```

### Q&A
```json
{
  "_id": "ObjectId",
  "question": "String",
  "answer": "String",
  "tags": ["String"],
  "novel_id": "ObjectId"
}
```

## 🤖 MCP Methods

The server supports the following MCP methods for LLM interaction:

### Query Methods

| Method | Description | Parameters |
|--------|-------------|------------|
| `query_character` | Get character details | `{"character_id": "string"}` |
| `query_novel` | Get novel metadata | `{"novel_id": "string"}` |
| `query_chapter` | Get chapter information | `{"chapter_id": "string"}` or `{"chapter_number": number}` or `{"chapter_title": "string"}` |
| `query_qa_regex` | Find Q&A entries by regex | `{"regex_pattern": "string"}` |
| `query_chapter_regex` | Find chapters by regex | `{"regex_pattern": "string"}` |
| `query_character_regex` | Find characters by regex | `{"regex_pattern": "string"}` |

### Update Methods

| Method | Description | Parameters |
|--------|-------------|------------|
| `update_chapter_summary` | Update chapter summary | `{"auth_token": "string", "chapter_id": "string", "summary": "string"}` |

## 📥 Data Population

This project includes Python scrapers as submodules to help populate your MongoDB:

### Character Scraper
```bash
cd character-scraper
pip install -r requirements.txt
python src/scraper.py --config config/settings.json
```

### Novel Scraper Library
```bash
cd scraper_library
pip install -r requirements.txt
python -m src.scrapers.scrape_syosetu --url <novel_url>
```

Available scrapers include:
- `scrape_syosetu.py` (Syosetu novels)
- `scrape_69shu.py` (69Shu novels)
- `scrape_ximalaya.py` (Ximalaya audio books)
- `scrape_qa.py` (Q&A content)
- `scrape_x_timeline.py` (Twitter/X timelines)
- And more...

## 🛠️ Development

### Updating Submodules

```bash
# Update all submodules to their latest version
git submodule update --remote --merge

# Navigate to a specific submodule and pull changes
cd scraper_library
git pull origin main
cd ..
git add scraper_library
git commit -m "Update scraper_library"
```

### Running Tests

```bash
cargo test
```

### Code Style

This project follows Rust's standard code style guidelines:

```bash
# Check code formatting
cargo fmt -- --check

# Run linting
cargo clippy
```

### Performance Tuning

For high-traffic deployments, consider:
1. MongoDB indexing for text search and relationships
2. Increase Tokio worker threads for CPU-bound operations
3. Configure MongoDB connection pooling

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.