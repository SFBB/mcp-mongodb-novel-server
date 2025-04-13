"""
Scraper Interface Definitions

This module defines the interfaces that all scrapers must implement.
"""

from abc import ABC, abstractmethod
from typing import List, Dict, Any

class Scraper(ABC):
    """Base interface for all scraper types."""
    
    @abstractmethod
    def get_source_info(self) -> Dict[str, str]:
        """Return information about the source website."""
        pass

class NovelScraper(Scraper):
    """Interface for text-based novel scrapers."""
    
    @abstractmethod
    def get_novel_info(self, url: str) -> Dict[str, Any]:
        """
        Get novel metadata from the URL.
        
        Args:
            url: URL of the novel's main page
            
        Returns:
            Dictionary containing at least 'title' and 'author'
        """
        pass
    
    @abstractmethod
    def get_chapter_list(self, url: str) -> List[str]:
        """
        Get a list of chapter URLs for the novel.
        
        Args:
            url: URL of the novel's main page or chapter list
            
        Returns:
            List of URLs for individual chapters
        """
        pass
    
    @abstractmethod
    def get_chapter_content(self, url: str) -> Dict[str, Any]:
        """
        Get content from a chapter URL.
        
        Args:
            url: URL of a chapter
            
        Returns:
            Dictionary with at least 'title' and 'content' keys
        """
        pass

class AudioNovelScraper(NovelScraper):
    """Interface for audio novel scrapers."""
    
    @abstractmethod
    def get_chapter_audio(self, url: str) -> Dict[str, str]:
        """
        Get audio file URL from a chapter URL.
        
        Args:
            url: URL of a chapter
            
        Returns:
            Dictionary with 'title' and 'audio_url' keys
        """
        pass