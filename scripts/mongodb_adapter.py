"""
MongoDB Data Adapter for MCP Database

This module connects the scraper_library with our MCP MongoDB server API.
It converts scraped novel data into our optimized schema and sends it to the database.
"""

import os
import json
import requests
from typing import Dict, List, Optional, Any
from datetime import datetime
import logging
import re

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("mongodb_adapter")

class MCPDatabaseAdapter:
    """Adapter to convert scraped data to our MCP database schema and populate the database."""
    
    def __init__(self, api_base_url: str = "http://localhost:3000"):
        """Initialize the adapter with the API base URL."""
        self.api_base_url = api_base_url
        self.session = requests.Session()
    
    def _make_request(self, method: str, endpoint: str, data: Optional[Dict] = None) -> Dict:
        """Make a request to the API."""
        url = f"{self.api_base_url}{endpoint}"
        try:
            if method.lower() == 'get':
                response = self.session.get(url)
            elif method.lower() == 'post':
                response = self.session.post(url, json=data)
            elif method.lower() == 'patch':
                response = self.session.patch(url, json=data)
            elif method.lower() == 'delete':
                response = self.session.delete(url)
            else:
                raise ValueError(f"Unsupported method: {method}")
            
            response.raise_for_status()
            return response.json()
        except requests.exceptions.RequestException as e:
            logger.error(f"API request error: {e}")
            raise
    
    def create_novel(self, novel_data: Dict) -> str:
        """Create a novel entry in the database."""
        logger.info(f"Creating novel: {novel_data.get('title', 'Unknown')}")
        response = self._make_request('post', '/api/novels', novel_data)
        return response.get('id')
    
    def create_chapter(self, chapter_data: Dict) -> str:
        """Create a chapter entry in the database."""
        logger.info(f"Creating chapter: {chapter_data.get('title', 'Unknown')}")
        response = self._make_request('post', '/api/chapters', chapter_data)
        return response.get('id')
    
    def create_character(self, character_data: Dict) -> str:
        """Create a character entry in the database."""
        logger.info(f"Creating character: {character_data.get('name', 'Unknown')}")
        response = self._make_request('post', '/api/characters', character_data)
        return response.get('id')
    
    def create_qa(self, qa_data: Dict) -> str:
        """Create a QA entry in the database."""
        logger.info(f"Creating QA entry: {qa_data.get('question', 'Unknown')}")
        response = self._make_request('post', '/api/qa', qa_data)
        return response.get('id')
    
    def _extract_key_points(self, text: str, max_points: int = 5) -> List[str]:
        """Extract key points from text content for token efficiency."""
        # Simple extraction - using sentences with important indicators
        # In a real implementation, you might use NLP techniques or AI for better extraction
        
        # Split into sentences
        sentences = re.split(r'(?<=[.!?])\s+', text)
        
        # Look for sentences with indicators of important events
        important_indicators = [
            'suddenly', 'finally', 'however', 'discovered', 'revealed', 
            'decided', 'realized', 'important', 'key', 'critical',
            'surprising', 'unexpected', 'secret', 'plan', 'attack',
            'victory', 'defeat', 'journey', 'arrive', 'leave'
        ]
        
        key_sentences = []
        for sentence in sentences:
            if any(indicator in sentence.lower() for indicator in important_indicators):
                key_sentences.append(sentence.strip())
        
        # If we don't have enough, take sentences from the beginning, middle and end
        if len(key_sentences) < max_points and len(sentences) > max_points:
            # Get some from beginning
            beginning = sentences[:max(1, max_points//3)]
            # Get some from middle
            middle_idx = len(sentences) // 2
            middle = sentences[middle_idx:middle_idx + max(1, max_points//3)]
            # Get some from end
            end = sentences[-max(1, max_points//3):]
            
            # Combine unique sentences
            key_sentences = list(set(key_sentences + beginning + middle + end))
        
        # Limit to max_points
        return key_sentences[:max_points]
    
    def _create_summary(self, text: str, max_length: int = 200) -> str:
        """Create a short summary from text content for token efficiency."""
        # For demonstration - in a real implementation you might use NLP techniques
        # or an LLM to generate better summaries
        
        # Simple approach: take first few sentences
        sentences = re.split(r'(?<=[.!?])\s+', text)
        
        summary = ""
        for sentence in sentences:
            if len(summary) + len(sentence) <= max_length:
                summary += sentence + " "
            else:
                break
        
        return summary.strip()
    
    def _extract_tags(self, novel_title: str, novel_description: str) -> List[str]:
        """Extract relevant tags from novel metadata."""
        # Common genre keywords to look for
        genres = [
            "fantasy", "sci-fi", "romance", "mystery", "thriller", "horror",
            "adventure", "historical", "fiction", "non-fiction", "biography",
            "action", "comedy", "drama", "suspense", "western", "crime"
        ]
        
        # Extract genres that appear in the title or description
        tags = []
        combined_text = (novel_title + " " + novel_description).lower()
        
        for genre in genres:
            if genre in combined_text:
                tags.append(genre)
        
        # Make sure we have at least some tags
        if not tags:
            tags = ["fiction"]
        
        return tags
    
    def process_novel(self, 
                      title: str, 
                      author: str, 
                      description: str,
                      chapters: List[Dict[str, str]],
                      characters: Optional[List[Dict[str, Any]]] = None) -> Dict:
        """
        Process novel data into our optimized schema and store in the database.
        
        Parameters:
        - title: Novel title
        - author: Author name
        - description: Novel description/synopsis
        - chapters: List of chapter dictionaries with 'title' and 'content' keys
        - characters: Optional list of character dictionaries
        
        Returns:
        - Dictionary with created IDs
        """
        result = {"novel": None, "chapters": [], "characters": []}
        
        # Create tags from metadata
        tags = self._extract_tags(title, description)
        
        # Create a summary
        summary = self._create_summary(description)
        
        # Prepare novel data in our schema
        novel_data = {
            "title": title,
            "author": author,
            "summary": summary,
            "tags": tags,
            "metadata": {
                "publication_date": datetime.now().strftime("%Y-%m-%d"),
                "genre": tags,
                "word_count": sum(len(ch.get("content", "").split()) for ch in chapters),
                "language": "en"  # Assuming English for simplicity
            }
        }
        
        # Create the novel in the database
        novel_id = self.create_novel(novel_data)
        result["novel"] = novel_id
        
        # Process chapters
        for i, chapter in enumerate(chapters, 1):
            chapter_title = chapter.get("title", f"Chapter {i}")
            chapter_content = chapter.get("content", "")
            
            # Create chapter summary and key points for token efficiency
            chapter_summary = self._create_summary(chapter_content)
            key_points = self._extract_key_points(chapter_content)
            
            # Prepare chapter data in our schema
            chapter_data = {
                "novel_id": novel_id,
                "number": i,
                "title": chapter_title,
                "summary": chapter_summary,
                "key_points": key_points,
                "content": chapter_content
            }
            
            # Create the chapter in the database
            chapter_id = self.create_chapter(chapter_data)
            result["chapters"].append(chapter_id)
        
        # Process characters if provided
        if characters:
            for character in characters:
                # Extract character info
                name = character.get("name", "Unknown")
                role = character.get("role", "supporting")
                description = character.get("description", "")
                
                # Prepare character data in our schema
                character_data = {
                    "novel_id": novel_id,
                    "name": name,
                    "role": role,
                    "description": description,
                    "key_traits": character.get("traits", []),
                    "relationships": character.get("relationships", [])
                }
                
                # Create the character in the database
                character_id = self.create_character(character_data)
                result["characters"].append(character_id)
        
        # Create some QA entries based on novel content
        self._create_qa_entries(novel_id, title, summary, chapters)
        
        return result
    
    def _create_qa_entries(self, novel_id: str, title: str, summary: str, chapters: List[Dict[str, str]]):
        """Create some basic QA entries for the novel."""
        # Create a basic "what is this novel about" QA
        qa_data = {
            "novel_id": novel_id,
            "question": f"What is the novel '{title}' about?",
            "answer": summary,
            "tags": ["summary", "overview"]
        }
        self.create_qa(qa_data)
        
        # Create a QA about the number of chapters
        qa_data = {
            "novel_id": novel_id,
            "question": f"How many chapters are in the novel '{title}'?",
            "answer": f"The novel '{title}' has {len(chapters)} chapters.",
            "tags": ["structure", "chapters"]
        }
        self.create_qa(qa_data)
        
        # Create chapter-specific QAs for a few early chapters
        for i, chapter in enumerate(chapters[:3], 1):
            chapter_title = chapter.get("title", f"Chapter {i}")
            chapter_content = chapter.get("content", "")
            summary = self._create_summary(chapter_content)
            
            qa_data = {
                "novel_id": novel_id,
                "question": f"What happens in chapter {i} of '{title}'?",
                "answer": summary,
                "tags": ["chapter", f"chapter-{i}"]
            }
            self.create_qa(qa_data)