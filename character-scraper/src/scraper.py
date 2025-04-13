import requests
import logging
import json
import re
import time
import sys
import os
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional, Tuple
from bs4 import BeautifulSoup
from database import Database

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("character_scraper")

class CharacterScraper:
    """
    Scraper for extracting character information from various sources.
    Optimized for token efficiency and small context windows (3k tokens).
    """
    
    def __init__(self, config_path: str = "config/settings.json"):
        """Initialize the character scraper with configuration"""
        self.config = self._load_config(config_path)
        self.db = self._init_database()
        self.rate_limit = self.config.get("scraping", {}).get("rate_limit", 3)  # seconds between requests
        self.headers = {
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
        }
        
    def _load_config(self, config_path: str) -> Dict:
        """Load configuration from JSON file"""
        try:
            with open(config_path, 'r') as f:
                return json.load(f)
        except Exception as e:
            logger.error(f"Error loading config: {e}")
            return {
                "database": {"host": "localhost", "port": 27017, "database_name": "character_db"},
                "scraping": {"target_url": "https://example.com/characters", "rate_limit": 3}
            }
    
    def _init_database(self) -> Database:
        """Initialize database connection"""
        db_config = self.config.get("database", {})
        uri = f"mongodb://{db_config.get('host', 'localhost')}:{db_config.get('port', 27017)}"
        return Database(uri, db_config.get("database_name", "character_db"))
    
    def fetch_character_data(self, url: str) -> str:
        """Fetch HTML content from a URL with error handling and retries"""
        max_retries = 3
        for attempt in range(max_retries):
            try:
                response = requests.get(url, headers=self.headers, timeout=10)
                response.raise_for_status()
                # Respect rate limits
                time.sleep(self.rate_limit)
                return response.text
            except requests.RequestException as e:
                logger.warning(f"Attempt {attempt+1}/{max_retries} failed: {e}")
                if attempt < max_retries - 1:
                    # Exponential backoff
                    time.sleep(2 ** attempt)
                else:
                    raise Exception(f"Failed to fetch data from {url} after {max_retries} attempts")
    
    def parse_character_data(self, html_content: str, selectors: Dict[str, str]) -> List[Dict[str, Any]]:
        """
        Parse character data from HTML content using configurable selectors
        
        Args:
            html_content: HTML content to parse
            selectors: Dictionary of CSS selectors for different character attributes
                       e.g. {"container": ".character", "name": "h2", "description": "p.description"}
        
        Returns:
            List of character dictionaries
        """
        soup = BeautifulSoup(html_content, 'html.parser')
        characters = []
        
        # Get container elements
        container_selector = selectors.get("container", "div.character")
        containers = soup.select(container_selector)
        
        # If no containers found, try to parse the whole page
        if not containers:
            logger.warning(f"No character containers found using selector: {container_selector}")
            containers = [soup]
        
        for container in containers:
            try:
                # Extract character data using selectors
                name_selector = selectors.get("name", "h2")
                name_elem = container.select_one(name_selector)
                
                description_selector = selectors.get("description", "p.description")
                description_elem = container.select_one(description_selector)
                
                # Skip if essential elements are missing
                if not name_elem:
                    continue
                
                # Extract text content
                name = name_elem.get_text(strip=True)
                description = description_elem.get_text(strip=True) if description_elem else ""
                
                # Extract additional attributes if available
                attributes = {}
                for attr_name, attr_selector in selectors.items():
                    if attr_name not in ["container", "name", "description"]:
                        elem = container.select_one(attr_selector)
                        if elem:
                            attributes[attr_name] = elem.get_text(strip=True)
                
                # Create character dictionary
                character = {
                    "name": name,
                    "description": description,
                    "source_url": self.config.get("scraping", {}).get("target_url", ""),
                    "scraped_at": datetime.now().isoformat(),
                    **attributes
                }
                
                # Extract traits, relationships and key points
                character["key_traits"] = self._extract_traits(description)
                character["key_points"] = self._extract_key_points(description)
                character["relationships"] = self._extract_relationships(container, selectors.get("relationships", ""))
                
                characters.append(character)
                
            except Exception as e:
                logger.error(f"Error parsing character: {e}")
                continue
        
        return characters
    
    def _extract_traits(self, description: str) -> List[str]:
        """
        Extract key character traits from description
        Optimized for token efficiency
        """
        # Common trait descriptors
        trait_words = [
            "brave", "intelligent", "strong", "wise", "cunning", "loyal", "determined", 
            "ambitious", "charismatic", "mysterious", "honorable", "humble", "proud",
            "gentle", "fierce", "cautious", "ruthless", "compassionate", "calculating"
        ]
        
        traits = []
        
        # Look for explicit trait statements
        trait_patterns = [
            r"is (very |extremely |remarkably |notably |particularly |especially )?(\w+)",
            r"(very |extremely |remarkably |notably |particularly |especially )?(\w+) (character|person|individual)"
        ]
        
        for pattern in trait_patterns:
            matches = re.finditer(pattern, description.lower())
            for match in matches:
                trait = match.group(2)
                if trait in trait_words and trait not in traits:
                    traits.append(trait)
        
        # Direct search for trait words
        for trait in trait_words:
            if trait in description.lower() and trait not in traits:
                traits.append(trait)
        
        return traits[:5]  # Limit to top 5 traits for token efficiency
    
    def _extract_key_points(self, text: str, max_points: int = 3) -> List[str]:
        """
        Extract key points from text for token-efficient storage
        Similar to the implementation in the QA scraper but adapted for characters
        """
        # Split text into sentences
        sentences = re.split(r'(?<=[.!?])\s+', text)
        
        # Filter out very short sentences
        sentences = [s for s in sentences if len(s.split()) > 3]
        
        # If we have very few sentences, return them all
        if len(sentences) <= max_points:
            return sentences
        
        # Select sentences that likely contain key information
        key_indicators = [
            "important", "key", "primary", "mainly", "notably", 
            "particularly", "essential", "defining", "critical"
        ]
        
        scored_sentences = []
        for sentence in sentences:
            score = 0
            # Score based on key indicator words
            for indicator in key_indicators:
                if indicator.lower() in sentence.lower():
                    score += 2
                    
            # Score based on sentence position (first sentences often contain key info)
            if sentence == sentences[0]:
                score += 3
            elif sentence == sentences[1]:  
                score += 1
                
            # Score based on sentence length (not too short, not too long)
            words = len(sentence.split())
            if 5 <= words <= 20:
                score += 1
                
            scored_sentences.append((score, sentence))
            
        # Sort by score (descending) and take top max_points
        scored_sentences.sort(reverse=True, key=lambda x: x[0])
        key_points = [sentence for score, sentence in scored_sentences[:max_points]]
        
        return key_points
    
    def _extract_relationships(self, container, relationship_selector: str) -> List[Dict[str, str]]:
        """Extract character relationships if available"""
        relationships = []
        
        if not relationship_selector:
            return relationships
            
        rel_elements = container.select(relationship_selector)
        
        for elem in rel_elements:
            try:
                rel_text = elem.get_text(strip=True)
                # Try to identify relationship type and target
                parts = rel_text.split(":")
                if len(parts) >= 2:
                    rel_type = parts[0].strip()
                    rel_target = parts[1].strip()
                    relationships.append({
                        "type": rel_type,
                        "character": rel_target
                    })
            except Exception:
                continue
                
        return relationships
    
    def format_for_database(self, characters: List[Dict[str, Any]], novel_id: Optional[str] = None) -> List[Dict[str, Any]]:
        """
        Format character data for the database
        Optimized for MCP MongoDB storage format
        """
        formatted_characters = []
        
        for character in characters:
            formatted_char = {
                "name": character["name"],
                "description": character["description"],
                "key_traits": character.get("key_traits", []),
                "key_points": character.get("key_points", []),
                "relationships": character.get("relationships", []),
                "source_url": character.get("source_url", ""),
                "scraped_at": character.get("scraped_at", datetime.now().isoformat())
            }
            
            # Add novel ID if provided
            if novel_id:
                formatted_char["novel_id"] = novel_id
                
            # Add role if it exists
            if "role" in character:
                formatted_char["role"] = character["role"]
                
            formatted_characters.append(formatted_char)
            
        return formatted_characters
    
    def scrape_characters(self, url: Optional[str] = None, selectors: Optional[Dict[str, str]] = None, novel_id: Optional[str] = None) -> List[Dict[str, Any]]:
        """
        Scrape characters from a URL and insert into database
        
        Args:
            url: URL to scrape (defaults to config URL if not provided)
            selectors: Dictionary of CSS selectors for different character attributes
            novel_id: Optional novel ID to associate with the characters
            
        Returns:
            List of inserted character IDs
        """
        target_url = url or self.config.get("scraping", {}).get("target_url")
        
        if not target_url:
            logger.error("No target URL specified")
            return []
            
        default_selectors = {
            "container": "div.character",
            "name": "h2",
            "description": "p.description"
        }
        
        # Use provided selectors or defaults
        char_selectors = selectors or default_selectors
        
        try:
            # Fetch and parse character data
            html_content = self.fetch_character_data(target_url)
            characters = self.parse_character_data(html_content, char_selectors)
            
            if not characters:
                logger.warning(f"No characters found at {target_url}")
                return []
                
            logger.info(f"Found {len(characters)} characters at {target_url}")
            
            # Format for database storage
            formatted_characters = self.format_for_database(characters, novel_id)
            
            # Insert into database
            inserted_ids = []
            for character in formatted_characters:
                character_id = self.db.insert_character(character)
                if character_id:
                    inserted_ids.append(str(character_id))
                    logger.info(f"Inserted character: {character['name']}")
            
            return inserted_ids
            
        except Exception as e:
            logger.error(f"Error scraping characters: {e}")
            return []
    
    def scrape_from_wiki(self, wiki_url: str, novel_id: Optional[str] = None) -> List[Dict[str, Any]]:
        """
        Specialized scraper for wiki-style character pages
        
        Args:
            wiki_url: URL of the wiki page with character information
            novel_id: Optional novel ID to associate with the characters
            
        Returns:
            List of inserted character IDs
        """
        try:
            # Wiki-specific selectors
            wiki_selectors = {
                "container": ".character-box, .char-info, .character-info",
                "name": "h3, .char-name, .character-name",
                "description": ".char-desc, .character-description, .description",
                "role": ".char-role, .character-role, .role",
                "relationships": ".char-relations li, .character-relations li"
            }
            
            return self.scrape_characters(wiki_url, wiki_selectors, novel_id)
            
        except Exception as e:
            logger.error(f"Error scraping wiki page: {e}")
            return []
    
    def cleanup(self):
        """Close database connection"""
        if hasattr(self, 'db'):
            self.db.close()

# Function interface for backwards compatibility
def fetch_character_data(url):
    scraper = CharacterScraper()
    return scraper.fetch_character_data(url)

def parse_character_data(html_content):
    scraper = CharacterScraper()
    return scraper.parse_character_data(html_content, {
        "container": "div.character",
        "name": "h2",
        "description": "p.description"
    })

def scrape_characters(url=None):
    try:
        scraper = CharacterScraper()
        return scraper.scrape_characters(url)
    finally:
        scraper.cleanup()

if __name__ == "__main__":
    try:
        # Parse command line arguments
        import argparse
        parser = argparse.ArgumentParser(description="Scrape character information for the MCP MongoDB database")
        parser.add_argument("--url", help="URL to scrape characters from")
        parser.add_argument("--novel-id", help="Novel ID to associate with the characters")
        parser.add_argument("--wiki", action="store_true", help="Use wiki-specific scraping rules")
        parser.add_argument("--config", default="config/settings.json", help="Path to config file")
        
        args = parser.parse_args()
        
        # Initialize scraper
        scraper = CharacterScraper(args.config)
        
        # Scrape characters
        if args.wiki and args.url:
            inserted_ids = scraper.scrape_from_wiki(args.url, args.novel_id)
        else:
            inserted_ids = scraper.scrape_characters(args.url, None, args.novel_id)
            
        if inserted_ids:
            print(f"Successfully inserted {len(inserted_ids)} characters")
        else:
            print("No characters inserted")
            
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)
    finally:
        if 'scraper' in locals():
            scraper.cleanup()