"""
Text File Adapter Implementation

This module implements a concrete adapter for saving novels as text files.
"""

import os
import logging
from typing import List, Dict, Any
from pathlib import Path

from scripts.scraper.adapter import TextFileAdapter

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("text_file_adapter")

class SimpleTextFileAdapter(TextFileAdapter):
    """
    Concrete adapter for saving novels as simple text files.
    """
    
    def __init__(self, output_path: str):
        """
        Initialize the adapter with the output file path.
        
        Args:
            output_path: Path where the text file will be saved
        """
        super().__init__(output_path)
        self.output_path = output_path
        
        # Create directory if it doesn't exist
        output_dir = os.path.dirname(output_path)
        if output_dir and not os.path.exists(output_dir):
            os.makedirs(output_dir)
    
    def format_novel_header(self, novel_info: Dict[str, Any]) -> str:
        """Format novel metadata for text file header."""
        title = novel_info.get("title", "Unknown Title")
        author = novel_info.get("author", "Unknown Author")
        description = novel_info.get("description", "")
        
        header = f"{title}\n"
        header += "=" * len(title) + "\n\n"
        header += f"Author: {author}\n\n"
        
        if description:
            header += "Description:\n"
            header += "-" * 12 + "\n"
            header += description + "\n\n"
        
        header += "=" * 50 + "\n\n"
        return header
    
    def format_chapter(self, chapter: Dict[str, Any]) -> str:
        """Format chapter content for text file."""
        title = chapter.get("title", "Chapter")
        content = chapter.get("content", "")
        
        formatted = f"\n\n{title}\n"
        formatted += "-" * len(title) + "\n\n"
        formatted += content + "\n\n"
        formatted += "=" * 50 + "\n"
        
        return formatted
    
    def process_novel(self, novel_info: Dict[str, Any], chapters: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Process novel information and chapter content into a text file.
        
        Args:
            novel_info: Dictionary containing novel metadata
            chapters: List of dictionaries containing chapter data
            
        Returns:
            Dictionary with file information
        """
        logger.info(f"Saving novel '{novel_info.get('title')}' to {self.output_path}")
        
        try:
            # Write novel header
            with open(self.output_path, 'w', encoding='utf-8') as f:
                header = self.format_novel_header(novel_info)
                f.write(header)
            
            # Write chapters
            for i, chapter in enumerate(chapters, 1):
                with open(self.output_path, 'a', encoding='utf-8') as f:
                    formatted_chapter = self.format_chapter(chapter)
                    f.write(formatted_chapter)
                logger.info(f"Added chapter {i}/{len(chapters)}")
            
            # Calculate some stats
            word_count = sum(len(ch.get("content", "").split()) for ch in chapters)
            
            logger.info(f"Novel saved successfully to {self.output_path}")
            logger.info(f"Total chapters: {len(chapters)}")
            logger.info(f"Total word count: {word_count}")
            
            return {
                "file_path": self.output_path,
                "chapters": len(chapters),
                "word_count": word_count
            }
            
        except Exception as e:
            logger.error(f"Error saving novel to text file: {e}")
            raise