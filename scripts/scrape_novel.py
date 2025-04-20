#!/usr/bin/env python
"""
Novel Scraper for MCP MongoDB Server

This script leverages the scraper_library to harvest novel data from public websites
and populate the MCP MongoDB server with optimized content.

The script focuses on token efficiency for small context windows (3k tokens)
by creating summaries, extracting key points, and formatting data appropriately.
"""

import os
import sys
import argparse
import logging
from typing import List, Dict, Optional, Any
import re
import time
from pathlib import Path

# Add scraper_library to path
SCRIPT_DIR = Path(__file__).resolve().parent
SCRAPER_LIB_DIR = SCRIPT_DIR.parent / "scraper_library"
sys.path.append(str(SCRAPER_LIB_DIR))

# Import the scraper_library modules
try:
    # Import base components
    from src.interfaces import Scraper, NovelScraper
    # Import specific scrapers from the correct paths
    from src.scrapers.scraper_69shu import Scraper69Shu
    from src.scrapers.scraper_baobao88 import ScraperBaobao88 as ScraperBaobao
    from src.scrapers.scraper_quanben import ScraperQuanben
    from src.scrapers.scraper_syosetu import ScraperSyosetu
    from src.scrapers.scraper_ximalaya import ScraperXimalaya
except ImportError as e:
    print(f"Error importing scraper_library: {e}")
    print("Make sure the scraper_library submodule is initialized.")
    print("Run: git submodule update --init --recursive")
    sys.exit(1)

# Import our MongoDB adapter
from mongodb_adapter import MCPDatabaseAdapter

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("novel_scraper")

# Available scrapers
SCRAPERS = {
    "69shu": Scraper69Shu,
    "baobao88": ScraperBaobao,
    "quanben": ScraperQuanben,
    "syosetu": ScraperSyosetu,
    "ximalaya": ScraperXimalaya
}

def extract_characters_from_content(novel_title: str, 
                                   chapters: List[Dict[str, str]]) -> List[Dict[str, Any]]:
    """
    Extract potential character information from chapter content.
    This is a simple implementation - a more sophisticated approach would use NLP.
    
    Parameters:
    - novel_title: Title of the novel for context
    - chapters: List of chapter dictionaries with content
    
    Returns:
    - List of character dictionaries
    """
    # Get the full content to analyze
    full_content = ""
    for chapter in chapters[:5]:  # Use the first few chapters for character detection
        full_content += chapter.get("content", "") + " "
    
    # Simple regex-based character extraction
    # Look for character name patterns (e.g., "Name said", "Name walked", etc.)
    name_patterns = [
        r'([A-Z][a-z]+ [A-Z][a-z]+)(?:\s+(?:said|spoke|asked|replied|answered|shouted|whispered|exclaimed|thought))',
        r'([A-Z][a-z]+)(?:\s+(?:said|spoke|asked|replied|answered|shouted|whispered|exclaimed|thought))',
        r'(?:Mr\.|Mrs\.|Ms\.|Dr\.|Professor) ([A-Z][a-z]+)',
    ]
    
    # Extract potential character names
    potential_names = []
    for pattern in name_patterns:
        matches = re.findall(pattern, full_content)
        potential_names.extend(matches)
    
    # Count occurrences to identify major characters (more mentions = more important)
    character_counts = {}
    for name in potential_names:
        if isinstance(name, tuple):
            name = name[0]  # For multi-group matches
        name = name.strip()
        if len(name) > 2:  # Avoid short acronyms
            character_counts[name] = character_counts.get(name, 0) + 1
    
    # Filter to most mentioned characters
    main_characters = sorted(character_counts.items(), key=lambda x: x[1], reverse=True)[:10]
    
    # Create character objects
    characters = []
    roles = ["protagonist", "antagonist", "supporting", "supporting", "supporting"]
    
    for i, (name, count) in enumerate(main_characters):
        # Skip if name is similar to the novel title (often not a character)
        if name.lower() in novel_title.lower() or novel_title.lower() in name.lower():
            continue
            
        # Assign a role based on mention frequency
        role = roles[min(i, len(roles)-1)]
        
        # Create a simple description
        if i == 0:
            description = f"Main character in the novel '{novel_title}'. Appears frequently throughout the story."
        elif i == 1 and role == "antagonist":
            description = f"Antagonist in the novel '{novel_title}'. Opposes the main character."
        else:
            description = f"Supporting character in the novel '{novel_title}'."
        
        # Extract sentences mentioning this character for traits
        character_sentences = []
        for chapter in chapters[:5]:
            content = chapter.get("content", "")
            sentences = re.split(r'(?<=[.!?])\s+', content)
            for sentence in sentences:
                if re.search(r'\b' + re.escape(name) + r'\b', sentence):
                    character_sentences.append(sentence)
        
        # Extract potential traits from sentences
        traits = []
        trait_words = [
            "brave", "smart", "intelligent", "kind", "cruel", "gentle", "fierce",
            "strong", "weak", "clever", "foolish", "wise", "naive", "loyal", "treacherous",
            "honest", "deceptive", "young", "old", "beautiful", "handsome", "ugly"
        ]
        
        for sentence in character_sentences[:20]:  # Limit to avoid overanalysis
            for trait in trait_words:
                if trait in sentence.lower():
                    traits.append(trait)
        
        # Deduplicate traits
        traits = list(set(traits))[:5]  # Limit to 5 traits for token efficiency
        
        characters.append({
            "name": name,
            "role": role,
            "description": description,
            "traits": traits,
            "relationships": []  # Would require more sophisticated analysis
        })
    
    return characters

def scrape_novel_to_database(scraper_name: str, novel_url: str, api_url: str) -> Dict:
    """
    Scrape a novel from the given URL using the specified scraper, 
    then populate the MCP MongoDB database.
    
    Parameters:
    - scraper_name: Name of the scraper to use
    - novel_url: URL of the novel to scrape
    - api_url: URL of the MCP MongoDB server API
    
    Returns:
    - Dictionary with results
    """
    # Check if the scraper exists
    if scraper_name not in SCRAPERS:
        raise ValueError(f"Scraper '{scraper_name}' not found. Available scrapers: {', '.join(SCRAPERS.keys())}")
    
    # Create the scraper instance
    scraper_class = SCRAPERS[scraper_name]
    scraper = scraper_class()
    
    logger.info(f"Starting scraping from {novel_url} using {scraper_name} scraper")
    
    # Extract novel info
    try:
        # Get novel metadata
        novel_info = scraper.get_novel_info(novel_url)
        title = novel_info.get("title", "Unknown Title")
        author = novel_info.get("author", "Unknown Author")
        description = novel_info.get("description", "")
        
        logger.info(f"Novel info: '{title}' by {author}")
        
        # Get chapter URLs
        chapter_urls = scraper.get_chapter_list(novel_url)
        logger.info(f"Found {len(chapter_urls)} chapters")
        
        # Create database adapter
        db_adapter = MCPDatabaseAdapter(api_url)
        
        # Scrape chapters (with limit to avoid overloading servers)
        max_chapters = min(50, len(chapter_urls))  # Limit to 50 chapters for demo purposes
        chapters = []
        
        for i, url in enumerate(chapter_urls[:max_chapters]):
            logger.info(f"Scraping chapter {i+1}/{max_chapters}")
            try:
                chapter_content = scraper.get_chapter_content(url)
                chapters.append({
                    "title": chapter_content.get("title", f"Chapter {i+1}"),
                    "content": chapter_content.get("content", "")
                })
                # Be nice to the server
                time.sleep(1)
            except Exception as e:
                logger.error(f"Error scraping chapter {i+1}: {e}")
        
        # Extract character information
        logger.info("Analyzing content to extract character information")
        characters = extract_characters_from_content(title, chapters)
        logger.info(f"Identified {len(characters)} potential characters")
        
        # Process and store everything in the database
        logger.info("Processing data and populating database")
        result = db_adapter.process_novel(
            title=title,
            author=author,
            description=description,
            chapters=chapters,
            characters=characters
        )
        
        logger.info(f"Successfully processed novel '{title}'. Created {len(result['chapters'])} chapters and {len(result['characters'])} characters")
        return result
        
    except Exception as e:
        logger.error(f"Error during scraping: {e}")
        raise

def main():
    """Main entry point for the script."""
    parser = argparse.ArgumentParser(description="Scrape novels and populate the MCP MongoDB database")
    parser.add_argument("--scraper", "-s", choices=SCRAPERS.keys(), required=True,
                       help="The scraper to use")
    parser.add_argument("--url", "-u", required=True,
                       help="URL of the novel to scrape")
    parser.add_argument("--api", "-a", default="http://localhost:3001",
                       help="URL of the MCP MongoDB server API (default: http://localhost:3001)")
    args = parser.parse_args()
    
    try:
        result = scrape_novel_to_database(args.scraper, args.url, args.api)
        print("Scraping and database population complete!")
        print(f"Novel ID: {result['novel']}")
        print(f"Created {len(result['chapters'])} chapters")
        print(f"Created {len(result['characters'])} characters")
    except Exception as e:
        logger.error(f"Error in main function: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()