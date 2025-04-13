#!/usr/bin/env python
"""
Character Info Scraper for MCP MongoDB Server

This script leverages the character-scraper to extract character information
from various sources and populate the MCP MongoDB server.

It focuses on token efficiency for small context windows (3k tokens)
by extracting key traits, relationships, and key points from descriptions.
"""

import os
import sys
import argparse
import logging
import json
from typing import List, Dict, Any, Optional
from pathlib import Path
import time
import requests

# Add character-scraper to path
SCRIPT_DIR = Path(__file__).resolve().parent
CHARACTER_SCRAPER_DIR = SCRIPT_DIR.parent / "character-scraper"
sys.path.append(str(CHARACTER_SCRAPER_DIR / "src"))

# Import modules
try:
    from scraper import CharacterScraper
except ImportError as e:
    print(f"Error importing modules: {e}")
    print(f"Make sure character-scraper is properly set up")
    sys.exit(1)

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler("character_scraper.log"),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger("scrape_characters")

class MCPDatabaseAdapter:
    """Adapter to connect to the MCP MongoDB API"""
    
    def __init__(self, api_base_url: str = "http://localhost:3000"):
        """Initialize the adapter with the API base URL"""
        self.api_base_url = api_base_url
        self.session = requests.Session()
        self.session.headers.update({
            "Content-Type": "application/json",
            "Accept": "application/json"
        })
    
    def create_character(self, character_data: Dict[str, Any]) -> Optional[str]:
        """Create a character entry in the database"""
        try:
            response = self.session.post(
                f"{self.api_base_url}/api/characters",
                json=character_data
            )
            response.raise_for_status()
            return response.json().get("_id")
        except Exception as e:
            logger.error(f"Error creating character: {e}")
            return None
    
    def get_novel(self, novel_id: str) -> Optional[Dict[str, Any]]:
        """Get novel details to verify the novel exists"""
        try:
            response = self.session.get(f"{self.api_base_url}/api/novels/{novel_id}")
            response.raise_for_status()
            return response.json()
        except Exception as e:
            logger.error(f"Error retrieving novel: {e}")
            return None

def save_to_mcp_database(characters: List[Dict[str, Any]], api_url: str) -> int:
    """Save character data to the MCP database via API"""
    adapter = MCPDatabaseAdapter(api_url)
    success_count = 0
    
    for character in characters:
        # Check if novel ID exists and is valid
        if "novel_id" in character:
            novel = adapter.get_novel(character["novel_id"])
            if not novel:
                logger.warning(f"Novel ID {character['novel_id']} not found. Character will not be linked to a novel.")
        
        # Create character in database
        character_id = adapter.create_character(character)
        if character_id:
            success_count += 1
            logger.info(f"Created character: {character['name']} (ID: {character_id})")
        else:
            logger.warning(f"Failed to create character: {character['name']}")
    
    return success_count

def save_to_json(characters: List[Dict[str, Any]], output_file: str) -> bool:
    """Save character data to a JSON file"""
    try:
        # Ensure the output directory exists
        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(characters, f, ensure_ascii=False, indent=2)
            
        logger.info(f"Saved {len(characters)} characters to {output_file}")
        return True
    except Exception as e:
        logger.error(f"Error saving to JSON file: {e}")
        return False

def print_to_console(characters: List[Dict[str, Any]]):
    """Print character data to console in a readable format"""
    if not characters:
        logger.info("No characters found")
        return
    
    print(f"\n==== Found {len(characters)} Characters ====\n")
    
    for i, character in enumerate(characters, 1):
        print(f"--- Character {i}: {character['name']} ---")
        
        if 'role' in character:
            print(f"Role: {character['role']}")
            
        print(f"Description: {character['description'][:150]}..." if len(character['description']) > 150 
              else f"Description: {character['description']}")
        
        if 'key_traits' in character and character['key_traits']:
            print("\nKey Traits:")
            for trait in character['key_traits']:
                print(f"- {trait}")
        
        if 'key_points' in character and character['key_points']:
            print("\nKey Points:")
            for point in character['key_points']:
                print(f"- {point}")
                
        if 'relationships' in character and character['relationships']:
            print("\nRelationships:")
            for rel in character['relationships']:
                print(f"- {rel.get('type', 'Related to')}: {rel.get('character', 'Unknown')}")
        
        print(f"\nSource: {character.get('source_url', 'Unknown')}")
        print("-" * 50)
        print()

def main():
    """Main entry point for the script"""
    parser = argparse.ArgumentParser(description="Scrape character information and populate MCP MongoDB database")
    
    # Source selection arguments
    parser.add_argument("--url", required=True, help="URL to scrape characters from")
    parser.add_argument("--novel-id", help="Novel ID to associate with the characters")
    
    # Scraping options
    parser.add_argument("--wiki", action="store_true", help="Use wiki-specific scraping rules")
    parser.add_argument("--rate-limit", type=int, default=3, 
                        help="Seconds to wait between requests (default: 3)")
    
    # Selector customization
    parser.add_argument("--container", help="CSS selector for character containers")
    parser.add_argument("--name", help="CSS selector for character names")
    parser.add_argument("--description", help="CSS selector for character descriptions")
    parser.add_argument("--role", help="CSS selector for character roles")
    parser.add_argument("--relationships", help="CSS selector for character relationships")
    
    # Output format arguments
    parser.add_argument("--output", choices=["mongodb", "json", "console"], default="mongodb",
                        help="Output format: mongodb, json, or console")
    parser.add_argument("--api-url", default="http://localhost:3000",
                        help="MCP MongoDB server API URL (default: http://localhost:3000)")
    parser.add_argument("--output-file", help="Path for JSON output file")
    
    args = parser.parse_args()
    
    # Validate arguments
    if args.output == "json" and not args.output_file:
        import time
        timestamp = time.strftime("%Y%m%d_%H%M%S")
        args.output_file = f"characters_{timestamp}.json"
        logger.warning(f"No output file specified for JSON output, using: {args.output_file}")
    
    # Create custom selectors dictionary if any selectors are provided
    selectors = {}
    if args.container:
        selectors["container"] = args.container
    if args.name:
        selectors["name"] = args.name
    if args.description:
        selectors["description"] = args.description
    if args.role:
        selectors["role"] = args.role
    if args.relationships:
        selectors["relationships"] = args.relationships
    
    # Initialize scraper with custom config
    config = {
        "scraping": {
            "rate_limit": args.rate_limit
        }
    }
    
    scraper = CharacterScraper()
    scraper.rate_limit = args.rate_limit
    
    try:
        # Scrape characters
        characters = []
        
        if args.wiki:
            logger.info(f"Scraping characters from wiki: {args.url}")
            character_ids = scraper.scrape_from_wiki(args.url, args.novel_id)
            
            # If we want to output to MongoDB, we already inserted during scraping
            if args.output == "mongodb":
                print(f"Successfully inserted {len(character_ids)} characters to database")
                return
                
            # For other outputs, we need to get the characters from the database
            characters = []
            for character_id in character_ids:
                character = scraper.db.get_character(character_id)
                if character:
                    characters.append(character)
        else:
            logger.info(f"Scraping characters from: {args.url}")
            
            # If using MongoDB output and no custom selectors, use direct scraping
            if args.output == "mongodb" and not selectors:
                character_ids = scraper.scrape_characters(args.url, None, args.novel_id)
                print(f"Successfully inserted {len(character_ids)} characters to database")
                return
            
            # Otherwise, scrape but don't insert yet
            html_content = scraper.fetch_character_data(args.url)
            characters = scraper.parse_character_data(html_content, selectors or None)
            
            if characters:
                characters = scraper.format_for_database(characters, args.novel_id)
                logger.info(f"Found {len(characters)} characters")
            else:
                logger.warning(f"No characters found at {args.url}")
                return
        
        # Handle output based on format
        if args.output == "mongodb":
            success_count = save_to_mcp_database(characters, args.api_url)
            print(f"Successfully added {success_count} characters to the MCP database")
        elif args.output == "json":
            save_to_json(characters, args.output_file)
        elif args.output == "console":
            print_to_console(characters)
            
    except Exception as e:
        logger.error(f"Error: {e}")
        sys.exit(1)
    finally:
        scraper.cleanup()

if __name__ == "__main__":
    main()