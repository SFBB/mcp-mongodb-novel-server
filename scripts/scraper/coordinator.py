"""
Novel Scraper Coordinator

This module provides a coordinator that combines scrapers and adapters
using a composition pattern, decoupling the scraping and storage logic.
"""

import logging
import time
from typing import List, Dict, Any, Optional, Type
import sys
import os
from pathlib import Path

# Add project root to path
SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
sys.path.append(str(ROOT_DIR))

# Import our modules
from scripts.scraper.interface import NovelScraper, AudioNovelScraper
from scripts.scraper.adapter import NovelAdapter, AudioAdapter
from scripts.scraper.threading import BatchProcessor, ChunkedBatchProcessor, ProgressTracker

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("novel_coordinator")

class NovelScraperCoordinator:
    """
    Coordinates the scraping and processing of novels.
    
    Uses composition to combine scrapers, adapters, and threading.
    """
    
    def __init__(
        self, 
        scraper: NovelScraper, 
        adapter: NovelAdapter,
        max_workers: int = 4,
        rate_limit_delay: float = 1.0,
        max_chapters: Optional[int] = None
    ):
        """
        Initialize the coordinator.
        
        Args:
            scraper: The scraper to use for data retrieval
            adapter: The adapter to use for data processing and storage
            max_workers: Maximum number of concurrent threads
            rate_limit_delay: Delay between requests to avoid overloading the server
            max_chapters: Maximum number of chapters to process (None for all)
        """
        self.scraper = scraper
        self.adapter = adapter
        self.max_workers = max_workers
        self.rate_limit_delay = rate_limit_delay
        self.max_chapters = max_chapters
    
    def scrape_novel(self, novel_url: str) -> Any:
        """
        Scrape and process a novel.
        
        Args:
            novel_url: URL of the novel's main page
            
        Returns:
            Result from the adapter's processing
        """
        logger.info(f"Starting to scrape novel from {novel_url}")
        
        # Get novel information
        novel_info = self.scraper.get_novel_info(novel_url)
        logger.info(f"Retrieved novel info: '{novel_info.get('title')}' by {novel_info.get('author', 'Unknown')}")
        
        # Get chapter URLs
        chapter_urls = self.scraper.get_chapter_list(novel_url)
        total_chapters = len(chapter_urls)
        logger.info(f"Found {total_chapters} chapters")
        
        # Apply chapter limit if specified
        if self.max_chapters and self.max_chapters < total_chapters:
            logger.info(f"Limiting to {self.max_chapters} chapters")
            chapter_urls = chapter_urls[:self.max_chapters]
        
        # Define chapter scraping function for multi-threading
        def scrape_chapter(url: str) -> Dict[str, Any]:
            result = self.scraper.get_chapter_content(url)
            # Respect rate limiting
            time.sleep(self.rate_limit_delay)
            return result
        
        # Scrape chapters in parallel
        logger.info(f"Scraping {len(chapter_urls)} chapters with {self.max_workers} workers")
        processor = BatchProcessor(
            worker_func=scrape_chapter,
            items=chapter_urls,
            max_workers=self.max_workers,
            description="Scraping chapters"
        )
        chapters = processor.process()
        
        # Process novel data with the adapter
        logger.info("Processing novel with adapter")
        result = self.adapter.process_novel(novel_info, chapters)
        
        logger.info(f"Completed processing novel '{novel_info.get('title')}'")
        return result
    
    @classmethod
    def create(
        cls,
        scraper_class: Type[NovelScraper],
        adapter_class: Type[NovelAdapter],
        adapter_params: Dict[str, Any],
        **kwargs
    ) -> 'NovelScraperCoordinator':
        """
        Factory method to create a coordinator with the specified components.
        
        Args:
            scraper_class: The scraper class to instantiate
            adapter_class: The adapter class to instantiate
            adapter_params: Parameters for the adapter constructor
            **kwargs: Additional parameters for the coordinator
            
        Returns:
            Configured NovelScraperCoordinator instance
        """
        scraper = scraper_class()
        adapter = adapter_class(**adapter_params)
        return cls(scraper, adapter, **kwargs)

class AudioNovelScraperCoordinator(NovelScraperCoordinator):
    """Coordinator for audio novel scraping."""
    
    def __init__(
        self, 
        scraper: AudioNovelScraper, 
        novel_adapter: NovelAdapter,
        audio_adapter: AudioAdapter,
        max_workers: int = 4,
        rate_limit_delay: float = 1.0,
        max_chapters: Optional[int] = None
    ):
        """
        Initialize the audio novel coordinator.
        
        Args:
            scraper: The audio scraper to use
            novel_adapter: Adapter for novel metadata
            audio_adapter: Adapter for audio content
            max_workers: Maximum number of concurrent threads
            rate_limit_delay: Delay between requests to avoid overloading the server
            max_chapters: Maximum number of chapters to process (None for all)
        """
        super().__init__(scraper, novel_adapter, max_workers, rate_limit_delay, max_chapters)
        self.audio_adapter = audio_adapter
    
    def scrape_novel(self, novel_url: str) -> Any:
        """
        Scrape and process an audio novel.
        
        Args:
            novel_url: URL of the novel's main page
            
        Returns:
            Result from the adapter's processing
        """
        # Use the parent class method to get the novel and chapter metadata
        novel_result = super().scrape_novel(novel_url)
        
        # Additional processing for audio files would go here
        # This would use the audio_adapter and the scraper's get_chapter_audio method
        
        return novel_result