"""
Threading Module

This module provides threading functionality for parallel scraping tasks.
The threading logic is decoupled from scrapers and adapters.
"""

import logging
import threading
from typing import List, Dict, Any, Callable, TypeVar, Generic
from concurrent.futures import ThreadPoolExecutor, as_completed
from tqdm import tqdm

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("threading_module")

T = TypeVar('T')
R = TypeVar('R')

class ThreadWorker(Generic[T, R]):
    """
    Generic thread worker for parallel processing.
    
    This class handles parallel execution of tasks over a list of items.
    """
    
    def __init__(
        self, 
        items: List[T], 
        process_func: Callable[[T, int], R], 
        max_workers: int = 4,
        show_progress: bool = True,
        progress_desc: str = "Processing"
    ):
        """
        Initialize the thread worker.
        
        Args:
            items: List of items to process
            process_func: Function to process each item
            max_workers: Maximum number of worker threads
            show_progress: Whether to show progress bar
            progress_desc: Description for the progress bar
        """
        self.items = items
        self.process_func = process_func
        self.max_workers = min(max_workers, len(items))
        self.show_progress = show_progress
        self.progress_desc = progress_desc
        self.results: List[R] = []
    
    def _worker(self, item: T, index: int) -> R:
        """
        Worker function that processes a single item.
        
        Args:
            item: Item to process
            index: Index of the item
            
        Returns:
            Result of processing the item
        """
        try:
            return self.process_func(item, index)
        except Exception as e:
            logger.error(f"Error processing item {index}: {e}")
            raise
    
    def run(self) -> List[R]:
        """
        Run processing in parallel threads.
        
        Returns:
            List of results in the same order as input items
        """
        if not self.items:
            logger.warning("No items to process")
            return []
        
        # Single-threaded processing for small lists or when max_workers is 1
        if len(self.items) < 3 or self.max_workers == 1:
            logger.info(f"Processing {len(self.items)} items sequentially")
            
            if self.show_progress:
                pbar = tqdm(total=len(self.items), desc=self.progress_desc)
            
            results = []
            for i, item in enumerate(self.items):
                result = self._worker(item, i)
                results.append(result)
                
                if self.show_progress:
                    pbar.update(1)
            
            if self.show_progress:
                pbar.close()
            
            self.results = results
            return results
        
        # Multi-threaded processing
        logger.info(f"Processing {len(self.items)} items with {self.max_workers} threads")
        
        # Create result placeholders
        results: List[R] = [None] * len(self.items)  # type: ignore
        
        with ThreadPoolExecutor(max_workers=self.max_workers) as executor:
            # Submit all tasks
            future_to_index = {
                executor.submit(self._worker, item, i): i 
                for i, item in enumerate(self.items)
            }
            
            # Track progress
            if self.show_progress:
                pbar = tqdm(total=len(self.items), desc=self.progress_desc)
                
            # Process results as they complete
            for future in as_completed(future_to_index):
                index = future_to_index[future]
                try:
                    result = future.result()
                    results[index] = result
                except Exception as e:
                    logger.error(f"Thread for item {index} raised an exception: {e}")
                    results[index] = None  # type: ignore
                
                if self.show_progress:
                    pbar.update(1)
            
            if self.show_progress:
                pbar.close()
        
        self.results = results
        return results