#!/usr/bin/env python
"""
X Timeline Scraper Script for MCP MongoDB Server

This script scrapes X (Twitter) timeline posts from specified users,
filters them by regex pattern, and populates the MCP MongoDB database.

Usage:
    python scrape_x_timeline.py --users username1,username2 --regex "pattern" [OPTIONS]

Options:
    --users USERNAME     Comma-separated list of usernames to scrape
    --regex PATTERN      Regex pattern to filter posts
    --api API_URL        URL of the MCP MongoDB server API (default: http://localhost:3000)
    --max-posts NUM      Maximum number of posts to retrieve per user (default: 200)
    --token TOKEN        X API bearer token (can also use X_BEARER_TOKEN env var)
    --output FORMAT      Output format: mongodb, json, or console (default: mongodb)
    --output-path PATH   Path for JSON output (if output=json)
"""

import os
import sys
import argparse
import logging
import json
import re
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional

# Add correct paths for imports
SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_DIR = SCRIPT_DIR.parent
SCRAPER_LIB_DIR = PROJECT_DIR / "scraper_library"
sys.path.append(str(SCRAPER_LIB_DIR))

# Import the scraper
try:
    from src.scrapers.scraper_x_timeline import ScraperXTimeline
except ImportError as e:
    print(f"Error importing scraper_library: {e}")
    print("Make sure the scraper_library is properly initialized")
    sys.exit(1)

# Import our MongoDB adapter
from mongodb_adapter import MCPDatabaseAdapter

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("x_timeline_scraper")

def process_x_timeline_data(scraped_data: Dict[str, Any]) -> Dict[str, Any]:
    """
    Process X timeline data into a format suitable for MCP MongoDB
    
    Args:
        scraped_data: Raw scraped data from X timeline
        
    Returns:
        Processed data ready for database insertion
    """
    username = scraped_data["username"]
    posts = scraped_data.get("posts", [])
    
    # Basic metadata
    result = {
        "title": f"X Timeline: @{username}",
        "author": username,
        "description": f"X posts from @{username}, filtered by regex pattern",
        "source_url": f"https://twitter.com/{username}",
        "post_count": len(posts),
        "scraped_at": scraped_data.get("scraped_at", datetime.now().isoformat()),
    }
    
    # Process posts as chapters for token efficiency
    # Each "chapter" contains a batch of posts (max 50 posts per chapter)
    chapters = []
    
    # Sort posts by date (newest first)
    sorted_posts = sorted(posts, key=lambda x: x.get("created_at", ""), reverse=True)
    
    # Group posts into batches (chapters)
    batch_size = 50
    for i in range(0, len(sorted_posts), batch_size):
        batch = sorted_posts[i:i+batch_size]
        
        # Create time range for chapter title
        start_date = batch[-1].get("created_at", "")[:10] if batch else ""
        end_date = batch[0].get("created_at", "")[:10] if batch else ""
        chapter_title = f"Posts from {start_date} to {end_date}"
        
        # Format posts into readable content
        content_parts = []
        for post in batch:
            post_date = post.get("created_at", "")[:10]
            post_time = post.get("created_at", "")[11:16] if post.get("created_at") else ""
            
            # Format the post with metadata
            post_text = f"[{post_date} {post_time}] {post.get('text', '')}"
            stats = f"♥ {post.get('like_count', 0)} | RT {post.get('retweet_count', 0)} | Replies {post.get('reply_count', 0)}"
            
            # Add hashtags if available
            hashtags = post.get("hashtags", [])
            if hashtags:
                hashtag_text = " ".join([f"#{tag}" for tag in hashtags])
                post_text += f"\nTags: {hashtag_text}"
            
            content_parts.append(f"{post_text}\n{stats}\n{'-' * 40}")
        
        chapter_content = "\n\n".join(content_parts)
        
        chapters.append({
            "title": chapter_title,
            "content": chapter_content
        })
    
    # Extract key topics and entities for character-like data
    topics = extract_topics_from_posts(posts)
    
    result["chapters"] = chapters
    result["topics"] = topics
    
    return result

def extract_topics_from_posts(posts: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """
    Extract key topics, hashtags, and mentioned users from posts
    This creates "character-like" entities for the MCP database
    
    Args:
        posts: List of post dictionaries
        
    Returns:
        List of topic dictionaries with topic data
    """
    # Extract hashtags
    hashtag_counts = {}
    for post in posts:
        for hashtag in post.get("hashtags", []):
            hashtag_counts[hashtag] = hashtag_counts.get(hashtag, 0) + 1
    
    # Extract mentions
    mention_counts = {}
    for post in posts:
        for mention in post.get("mentions", []):
            mention_counts[mention] = mention_counts.get(mention, 0) + 1
    
    # Create topic entities (similar to characters in novel context)
    topics = []
    
    # Add top hashtags
    top_hashtags = sorted(hashtag_counts.items(), key=lambda x: x[1], reverse=True)[:10]
    for hashtag, count in top_hashtags:
        topics.append({
            "name": f"#{hashtag}",
            "role": "hashtag",
            "description": f"Hashtag mentioned {count} times in the timeline",
            "traits": ["hashtag", "topic"]
        })
    
    # Add top mentions
    top_mentions = sorted(mention_counts.items(), key=lambda x: x[1], reverse=True)[:10]
    for mention, count in top_mentions:
        topics.append({
            "name": f"@{mention}",
            "role": "user",
            "description": f"User mentioned {count} times in the timeline",
            "traits": ["mention", "user"]
        })
    
    return topics

def save_to_mongodb(processed_data: Dict[str, Any], api_url: str) -> Dict:
    """
    Save processed data to MCP MongoDB server
    
    Args:
        processed_data: Processed data ready for database
        api_url: MCP MongoDB server API URL
        
    Returns:
        Result dictionary with database IDs
    """
    try:
        # Create database adapter
        db_adapter = MCPDatabaseAdapter(api_url)
        
        # Process and store everything in the database
        logger.info("Processing data and populating database")
        result = db_adapter.process_novel(
            title=processed_data["title"],
            author=processed_data["author"],
            description=processed_data["description"],
            chapters=processed_data["chapters"],
            characters=processed_data["topics"]
        )
        
        logger.info(f"Successfully processed X timeline data. Created {len(result['chapters'])} chapters and {len(result['characters'])} topic entities")
        return result
    except Exception as e:
        logger.error(f"Error saving to MongoDB: {e}")
        raise

def save_to_json(processed_data: Dict[str, Any], output_path: str) -> None:
    """
    Save processed data to JSON file
    
    Args:
        processed_data: Processed data
        output_path: Path to save JSON file
    """
    try:
        # Create directory if it doesn't exist
        os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
        
        # Save to file
        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(processed_data, f, ensure_ascii=False, indent=2)
        
        logger.info(f"Data saved to {output_path}")
    except Exception as e:
        logger.error(f"Error saving to JSON: {e}")
        raise

def main():
    """Main entry point for the script"""
    parser = argparse.ArgumentParser(description="Scrape X timeline posts and populate MCP MongoDB database")
    parser.add_argument("--users", required=True, help="Comma-separated list of usernames to scrape")
    parser.add_argument("--regex", required=True, help="Regex pattern to filter posts")
    parser.add_argument("--api", default="http://localhost:3000", help="MCP MongoDB server API URL")
    parser.add_argument("--max-posts", type=int, default=200, help="Maximum posts per user")
    parser.add_argument("--token", help="X API bearer token (can also use X_BEARER_TOKEN env var)")
    parser.add_argument("--output", choices=["mongodb", "json", "console"], default="mongodb", 
                        help="Output format: mongodb, json, or console")
    parser.add_argument("--output-path", help="Path for JSON output (if output=json)")
    args = parser.parse_args()
    
    # Check if bearer token is provided
    bearer_token = args.token or os.environ.get("X_BEARER_TOKEN")
    if not bearer_token:
        logger.error("X API bearer token is required. Provide it with --token or set X_BEARER_TOKEN environment variable")
        sys.exit(1)
    
    # Parse usernames
    usernames = [u.strip() for u in args.users.split(",")]
    if not usernames:
        logger.error("At least one username is required")
        sys.exit(1)
    
    try:
        # Create the scraper with bearer token
        scraper = ScraperXTimeline(bearer_token=bearer_token, max_results=args.max_posts)
        
        # Scrape user timelines with regex filtering
        logger.info(f"Scraping X timelines for {len(usernames)} users with regex filter: {args.regex}")
        results = scraper.scrape_multiple_pages(usernames, regex_filter=args.regex)
        
        # Process results
        for result in results:
            username = result["username"]
            post_count = result.get("post_count", 0)
            
            logger.info(f"Found {post_count} matching posts for @{username}")
            
            if post_count == 0:
                logger.warning(f"No matching posts found for @{username}")
                continue
            
            # Process data for storage
            processed_data = process_x_timeline_data(result)
            
            # Save according to output format
            if args.output == "mongodb":
                save_to_mongodb(processed_data, args.api)
            elif args.output == "json":
                if not args.output_path:
                    output_path = f"x_timeline_{username}_{datetime.now().strftime('%Y%m%d')}.json"
                else:
                    output_path = args.output_path
                
                save_to_json(processed_data, output_path)
            else:  # console
                print(json.dumps(processed_data, indent=2))
        
        logger.info("X timeline scraping complete")
        
    except Exception as e:
        logger.error(f"Error during scraping: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()