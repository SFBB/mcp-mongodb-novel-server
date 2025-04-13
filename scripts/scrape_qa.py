#!/usr/bin/env python
"""
Q&A Scraper for MCP MongoDB Server

This script leverages the scraper_library to extract Q&A content from X posts
and other sources, then populates the MCP MongoDB server with optimized content.

The script focuses on token efficiency for small context windows (3k tokens)
by creating summaries, extracting key points, and formatting data appropriately.
"""

import os
import sys
import argparse
import logging
from typing import List, Dict, Optional, Any
import json
from pathlib import Path
import time

# Add scraper_library to path
SCRIPT_DIR = Path(__file__).resolve().parent
SCRAPER_LIB_DIR = SCRIPT_DIR.parent / "scraper_library"
sys.path.append(str(SCRAPER_LIB_DIR))

# Import the scraper_library modules
try:
    from src.scrapers.scraper_qa import ScraperQA
    from scripts.mongodb_adapter import MCPDatabaseAdapter
except ImportError as e:
    print(f"Error importing required modules: {e}")
    print(f"Make sure scraper_library is properly initialized")
    sys.exit(1)

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler("qa_scraper.log"),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger("scrape_qa")

def validate_args(args):
    """Validate command line arguments"""
    if args.source == "x" and not args.usernames:
        logger.error("X scraping requires at least one username")
        return False
    elif args.source == "askfm" and not args.usernames:
        logger.error("Ask.fm scraping requires at least one username")
        return False
    elif args.source == "custom" and not (args.url and args.question_selector and args.answer_selector):
        logger.error("Custom site scraping requires URL, question selector, and answer selector")
        return False
    
    if args.output == "mongodb" and not args.api_url:
        logger.warning("No API URL provided for MongoDB output, using default: http://localhost:3000")
        args.api_url = "http://localhost:3000"
    elif args.output == "json" and not args.output_file:
        timestamp = time.strftime("%Y%m%d_%H%M%S")
        args.output_file = f"qa_data_{timestamp}.json"
        logger.warning(f"No output file specified for JSON output, using: {args.output_file}")
    
    return True

def scrape_from_x(scraper: ScraperQA, usernames: List[str], filter_pattern: str, novel_id: Optional[str]) -> List[Dict[str, Any]]:
    """Scrape Q&A content from X users"""
    all_qa_data = []
    
    for username in usernames:
        try:
            logger.info(f"Scraping Q&A from X user: {username}")
            qa_data = scraper.scrape_from_x(username, filter_pattern)
            
            if qa_data["qa_count"] > 0:
                logger.info(f"Found {qa_data['qa_count']} Q&A pairs from {username}")
                # Format for database if needed
                formatted_data = scraper.format_for_database(qa_data, novel_id)
                all_qa_data.extend(formatted_data)
            else:
                logger.warning(f"No Q&A content found for X user: {username}")
            
            # Respect rate limits
            time.sleep(scraper.rate_limit_wait)
            
        except Exception as e:
            logger.error(f"Error scraping X user {username}: {str(e)}")
    
    return all_qa_data

def scrape_from_askfm(scraper: ScraperQA, usernames: List[str], novel_id: Optional[str]) -> List[Dict[str, Any]]:
    """Scrape Q&A content from ask.fm users"""
    all_qa_data = []
    
    for username in usernames:
        try:
            logger.info(f"Scraping Q&A from ask.fm user: {username}")
            qa_data = scraper.scrape_from_askfm(username)
            
            if qa_data["qa_count"] > 0:
                logger.info(f"Found {qa_data['qa_count']} Q&A pairs from ask.fm user: {username}")
                # Format for database if needed
                formatted_data = scraper.format_for_database(qa_data, novel_id)
                all_qa_data.extend(formatted_data)
            else:
                logger.warning(f"No Q&A content found for ask.fm user: {username}")
            
            # Respect rate limits
            time.sleep(scraper.rate_limit_wait)
            
        except Exception as e:
            logger.error(f"Error scraping ask.fm user {username}: {str(e)}")
    
    return all_qa_data

def scrape_from_custom_site(scraper: ScraperQA, url: str, question_selector: str, 
                           answer_selector: str, novel_id: Optional[str]) -> List[Dict[str, Any]]:
    """Scrape Q&A content from a custom website"""
    try:
        logger.info(f"Scraping Q&A from custom site: {url}")
        qa_data = scraper.scrape_from_custom_site(url, question_selector, answer_selector)
        
        if qa_data["qa_count"] > 0:
            logger.info(f"Found {qa_data['qa_count']} Q&A pairs from {url}")
            # Format for database if needed
            formatted_data = scraper.format_for_database(qa_data, novel_id)
            return formatted_data
        else:
            logger.warning(f"No Q&A content found for custom site: {url}")
            return []
            
    except Exception as e:
        logger.error(f"Error scraping custom site {url}: {str(e)}")
        return []

def save_to_json(qa_data: List[Dict[str, Any]], output_file: str):
    """Save Q&A data to a JSON file"""
    try:
        # Ensure the output directory exists
        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(qa_data, f, ensure_ascii=False, indent=2)
            
        logger.info(f"Saved {len(qa_data)} Q&A entries to {output_file}")
        return True
    except Exception as e:
        logger.error(f"Error saving to JSON file: {str(e)}")
        return False

def save_to_mongodb(qa_data: List[Dict[str, Any]], api_url: str):
    """Save Q&A data to MongoDB via the API"""
    try:
        logger.info(f"Connecting to MCP MongoDB API at {api_url}")
        adapter = MCPDatabaseAdapter(api_base_url=api_url)
        
        success_count = 0
        error_count = 0
        
        for qa_entry in qa_data:
            try:
                # Create a Q&A entry in the database
                result = adapter.create_qa(qa_entry)
                if result:
                    success_count += 1
                else:
                    error_count += 1
            except Exception as e:
                logger.error(f"Error saving Q&A entry to MongoDB: {str(e)}")
                error_count += 1
        
        logger.info(f"Saved {success_count} Q&A entries to MongoDB ({error_count} errors)")
        return success_count > 0
    except Exception as e:
        logger.error(f"Error connecting to MongoDB API: {str(e)}")
        return False

def print_to_console(qa_data: List[Dict[str, Any]]):
    """Print Q&A data to console in a readable format"""
    if not qa_data:
        logger.info("No Q&A data found")
        return
    
    print(f"\n==== Found {len(qa_data)} Q&A Entries ====\n")
    
    for i, entry in enumerate(qa_data, 1):
        print(f"--- Entry {i} ---")
        print(f"Q: {entry['question']}")
        print(f"A: {entry['answer']}")
        
        if 'key_points' in entry and entry['key_points']:
            print("\nKey Points:")
            for point in entry['key_points']:
                print(f"- {point}")
        
        if 'tags' in entry and entry['tags']:
            print(f"\nTags: {', '.join(entry['tags'])}")
            
        print(f"Source: {entry.get('source', 'Unknown')}")
        print(f"Timestamp: {entry.get('timestamp', 'Unknown')}")
        print()

def main():
    """Main entry point for the script"""
    parser = argparse.ArgumentParser(description="Scrape Q&A content and populate MCP MongoDB database")
    
    # Source selection arguments
    parser.add_argument("--source", choices=["x", "askfm", "custom", "all"], default="x",
                        help="Source to scrape: x, askfm, custom, or all")
    
    # X-specific arguments
    parser.add_argument("--usernames", help="Comma-separated list of usernames to scrape")
    parser.add_argument("--filter", default="ask.fm", 
                        help="Regex pattern to filter X posts (default: 'ask.fm')")
    parser.add_argument("--token", help="X API bearer token (can also use X_BEARER_TOKEN env var)")
    
    # Custom site arguments
    parser.add_argument("--url", help="URL of custom Q&A site to scrape")
    parser.add_argument("--question-selector", help="CSS selector for question elements")
    parser.add_argument("--answer-selector", help="CSS selector for answer elements")
    
    # General arguments
    parser.add_argument("--novel-id", help="Optional novel ID to associate with Q&A data")
    parser.add_argument("--max-qa", type=int, default=100, 
                        help="Maximum number of Q&A pairs per source")
    parser.add_argument("--rate-limit", type=int, default=5,
                        help="Seconds to wait between requests (default: 5)")
    
    # Output format arguments
    parser.add_argument("--output", choices=["mongodb", "json", "console"], default="mongodb",
                        help="Output format: mongodb, json, or console")
    parser.add_argument("--api-url", default="http://localhost:3000",
                        help="MCP MongoDB server API URL (default: http://localhost:3000)")
    parser.add_argument("--output-file", help="Path for JSON output file")
    
    args = parser.parse_args()
    
    # Validate arguments
    if not validate_args(args):
        parser.print_help()
        sys.exit(1)
    
    # Initialize scraper
    scraper_kwargs = {
        "bearer_token": args.token or os.environ.get("X_BEARER_TOKEN"),
        "max_qa": args.max_qa,
        "rate_limit_wait": args.rate_limit
    }
    
    scraper = ScraperQA(**scraper_kwargs)
    
    # Process usernames
    usernames = []
    if args.usernames:
        usernames = [u.strip() for u in args.usernames.split(",")]
    
    # Scrape based on source
    all_qa_data = []
    
    if args.source == "x" or args.source == "all":
        if usernames:
            x_qa_data = scrape_from_x(scraper, usernames, args.filter, args.novel_id)
            all_qa_data.extend(x_qa_data)
    
    if args.source == "askfm" or args.source == "all":
        if usernames:
            askfm_qa_data = scrape_from_askfm(scraper, usernames, args.novel_id)
            all_qa_data.extend(askfm_qa_data)
    
    if args.source == "custom" or args.source == "all":
        if args.url and args.question_selector and args.answer_selector:
            custom_qa_data = scrape_from_custom_site(
                scraper, args.url, args.question_selector, args.answer_selector, args.novel_id
            )
            all_qa_data.extend(custom_qa_data)
    
    # Handle output
    if all_qa_data:
        logger.info(f"Successfully scraped {len(all_qa_data)} Q&A entries")
        
        if args.output == "mongodb":
            save_to_mongodb(all_qa_data, args.api_url)
        elif args.output == "json":
            save_to_json(all_qa_data, args.output_file)
        elif args.output == "console":
            print_to_console(all_qa_data)
    else:
        logger.warning("No Q&A data was scraped")
    
    logger.info("Q&A scraping completed")

if __name__ == "__main__":
    main()