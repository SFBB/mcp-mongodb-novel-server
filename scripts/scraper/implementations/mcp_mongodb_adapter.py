"""
MCP MongoDB Adapter Implementation

This module implements a concrete adapter for saving novels to MongoDB
using the Model Context Protocol (MCP).
"""

import json
import logging
import requests
from typing import List, Dict, Any, Optional
from datetime import datetime

from scripts.scraper.adapter import DatabaseAdapter

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message=s'
)
logger = logging.getLogger("mcp_mongodb_adapter")

class MCPMongoDBAdapter(DatabaseAdapter):
    """
    Adapter for saving novels to MongoDB using the MCP protocol.
    Optimized for small context windows (3k tokens).
    """
    
    def __init__(self, host: str, port: int, database: str = "novels", collection: str = "novels"):
        """
        Initialize the MCP MongoDB adapter.
        
        Args:
            host: MongoDB server host
            port: MongoDB server port
            database: Database name
            collection: Collection name
        """
        config = {
            "host": host,
            "port": port,
            "database": database,
            "collection": collection,
            "base_url": f"http://{host}:{port}"
        }
        super().__init__(config)
        self.base_url = config["base_url"]
        
    def _make_request(self, method: str, endpoint: str, data: Optional[Dict] = None) -> Dict:
        """
        Make an MCP protocol request to the MongoDB server.
        
        Args:
            method: HTTP method (GET, POST, etc.)
            endpoint: API endpoint
            data: Request data
            
        Returns:
            Response data
        """
        url = f"{self.base_url}{endpoint}"
        
        try:
            if method.upper() == "GET":
                response = requests.get(url)
            elif method.upper() == "POST":
                response = requests.post(url, json=data)
            elif method.upper() == "PUT":
                response = requests.put(url, json=data)
            elif method.upper() == "DELETE":
                response = requests.delete(url)
            else:
                raise ValueError(f"Unsupported HTTP method: {method}")
            
            response.raise_for_status()
            return response.json()
        except requests.exceptions.RequestException as e:
            logger.error(f"Error making request to {url}: {e}")
            raise
    
    def _chunk_content(self, text: str, max_tokens: int = 1000) -> List[str]:
        """
        Chunk large content into smaller pieces for better token efficiency.
        
        Args:
            text: Text to chunk
            max_tokens: Maximum tokens per chunk (approximately)
            
        Returns:
            List of content chunks
        """
        # Simple chunking by paragraphs to stay under token limit
        paragraphs = text.split("\n\n")
        chunks = []
        current_chunk = ""
        
        for paragraph in paragraphs:
            # Rough estimation: 1 token ~= 4 characters for English text
            estimated_tokens = len(current_chunk + paragraph) / 4
            
            if estimated_tokens > max_tokens and current_chunk:
                chunks.append(current_chunk)
                current_chunk = paragraph
            else:
                if current_chunk:
                    current_chunk += "\n\n" + paragraph
                else:
                    current_chunk = paragraph
        
        if current_chunk:
            chunks.append(current_chunk)
            
        return chunks
    
    def process_novel(self, novel_info: Dict[str, Any], chapters: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Process novel information and chapter content into MongoDB using MCP.
        
        Args:
            novel_info: Dictionary containing novel metadata
            chapters: List of dictionaries containing chapter data
            
        Returns:
            Dictionary with processing results
        """
        logger.info(f"Saving novel '{novel_info.get('title')}' to MongoDB with MCP")
        
        try:
            # Create novel document
            novel_data = {
                "title": novel_info.get("title", "Unknown Title"),
                "author": novel_info.get("author", "Unknown Author"),
                "description": novel_info.get("description", ""),
                "source_url": novel_info.get("source_url", ""),
                "language": novel_info.get("language", "unknown"),
                "genre": novel_info.get("genre", []),
                "chapter_count": len(chapters),
                "word_count": sum(len(ch.get("content", "").split()) for ch in chapters),
                "date_added": datetime.now().isoformat(),
                "metadata": {
                    "source_website": novel_info.get("source_website", ""),
                    "scraper_version": novel_info.get("scraper_version", "1.0.0"),
                }
            }
            
            # Create novel document via MCP
            novel_response = self._make_request(
                "POST", 
                "/api/novels", 
                {
                    "jsonrpc": "2.0",
                    "method": "createNovel",
                    "params": {
                        "novel": novel_data
                    },
                    "id": 1
                }
            )
            
            novel_id = novel_response.get("result", {}).get("id")
            if not novel_id:
                raise ValueError("Failed to create novel document")
            
            logger.info(f"Created novel document with ID: {novel_id}")
            
            # Add chapters with chunking for large content
            for i, chapter in enumerate(chapters):
                chapter_title = chapter.get("title", f"Chapter {i+1}")
                chapter_content = chapter.get("content", "")
                
                # Chunk large content for better token efficiency
                content_chunks = self._chunk_content(chapter_content)
                
                chapter_data = {
                    "novel_id": novel_id,
                    "title": chapter_title,
                    "chapter_number": i + 1,
                    "content": content_chunks[0] if content_chunks else "",
                    "content_chunks": content_chunks[1:] if len(content_chunks) > 1 else [],
                    "word_count": len(chapter_content.split()),
                }
                
                self._make_request(
                    "POST", 
                    "/api/chapters", 
                    {
                        "jsonrpc": "2.0",
                        "method": "createChapter",
                        "params": {
                            "chapter": chapter_data
                        },
                        "id": i + 2
                    }
                )
                
                logger.info(f"Added chapter {i+1}/{len(chapters)}: {chapter_title}")
            
            logger.info(f"Novel saved successfully to MongoDB with {len(chapters)} chapters")
            
            return {
                "novel_id": novel_id,
                "title": novel_data["title"],
                "chapters": len(chapters),
                "word_count": novel_data["word_count"]
            }
            
        except Exception as e:
            logger.error(f"Error saving novel to MongoDB: {e}")
            raise