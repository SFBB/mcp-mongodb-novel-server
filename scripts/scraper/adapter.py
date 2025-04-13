"""
Adapter Interface Definitions

This module defines the interfaces that all adapters must implement.
"""

from abc import ABC, abstractmethod
from typing import List, Dict, Any, Optional

class Adapter(ABC):
    """Base interface for all adapter types."""
    
    def __init__(self, config: Optional[Dict[str, Any]] = None):
        """
        Initialize the adapter with configuration.
        
        Args:
            config: Dictionary of configuration parameters
        """
        self.config = config or {}
    
    @abstractmethod
    def process_novel(self, novel_info: Dict[str, Any], chapters: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Process novel information and chapter content.
        
        Args:
            novel_info: Dictionary containing novel metadata
            chapters: List of dictionaries containing chapter data
            
        Returns:
            Dictionary with processing results
        """
        pass

class NovelAdapter(Adapter):
    """Base interface for text novel adapters."""
    pass

class AudioAdapter(Adapter):
    """Base interface for audio novel adapters."""
    
    @abstractmethod
    def process_audio(self, novel_info: Dict[str, Any], chapters: List[Dict[str, Any]]) -> Dict[str, Any]:
        """
        Process audio novel information and audio files.
        
        Args:
            novel_info: Dictionary containing novel metadata
            chapters: List of dictionaries containing chapter data and audio URLs
            
        Returns:
            Dictionary with processing results
        """
        pass

class TextFileAdapter(NovelAdapter):
    """Base class for text file adapters."""
    
    def __init__(self, file_path: str):
        """
        Initialize the text file adapter.
        
        Args:
            file_path: Path where the file will be saved
        """
        super().__init__({"file_path": file_path})
        self.file_path = file_path

class JSONAdapter(NovelAdapter):
    """Base class for JSON file adapters."""
    
    def __init__(self, file_path: str):
        """
        Initialize the JSON adapter.
        
        Args:
            file_path: Path where the JSON file will be saved
        """
        super().__init__({"file_path": file_path})
        self.file_path = file_path

class DatabaseAdapter(NovelAdapter):
    """Base class for database adapters."""
    
    def __init__(self, db_config: Dict[str, Any]):
        """
        Initialize the database adapter.
        
        Args:
            db_config: Dictionary with database connection parameters
        """
        super().__init__(db_config)
        
    @abstractmethod
    def _make_request(self, method: str, endpoint: str, data: Optional[Dict] = None) -> Dict:
        """Make a request to the database API."""
        pass