"""Abstract base class for all source syncs."""

from abc import ABC, abstractmethod
from typing import Any, Dict, Optional, List, Tuple
from datetime import datetime, timedelta, timezone


class BaseSync(ABC):
    """
    Enhanced abstract base class for all source sync implementations.
    
    Provides common functionality for initial vs incremental sync detection,
    date range calculation based on configuration, and standardized sync flow.
    """
    
    # Common retry configuration
    MAX_RETRIES = 3
    RETRY_DELAY = 1.0  # seconds
    
    def __init__(self, stream, access_token: Optional[str] = None, token_refresher=None):
        """
        Initialize the sync handler.
        
        Args:
            stream: Stream wrapper object with configuration
            access_token: Optional OAuth access token
            token_refresher: Optional token refresh callback
        """
        self.stream = stream
        self.access_token = access_token
        self.token_refresher = token_refresher
    
    def is_initial_sync(self) -> bool:
        """
        Determine if this is an initial sync.
        
        Returns:
            True if no previous successful sync exists
        """
        return (
            not hasattr(self.stream, 'last_successful_ingestion_at') or 
            self.stream.last_successful_ingestion_at is None
        )
    
    def get_sync_date_range(self) -> Tuple[Optional[datetime], Optional[datetime]]:
        """
        Calculate date range based on sync type and configuration.
        
        For initial syncs:
        - 'full': Uses source-specific full range (defined in subclass)
        - 'limited': Uses configured days from database
        
        For incremental syncs:
        - Uses source-specific incremental logic (sync tokens, last sync time, etc.)
        
        Returns:
            Tuple of (start_date, end_date), either can be None for token-based syncs
        """
        now = datetime.now(timezone.utc)
        
        if self.is_initial_sync():
            sync_type = getattr(self.stream, 'initial_sync_type', 'limited')
            
            if sync_type == 'full':
                # Use source-specific full sync range
                return self.get_full_sync_range()
            else:  # 'limited'
                # Use configured days from database
                days_past = getattr(self.stream, 'initial_sync_days', 90)
                days_future = getattr(self.stream, 'initial_sync_days_future', 30)
                
                print(f"Initial limited sync: {days_past} days past, {days_future} days future")
                
                return (
                    now - timedelta(days=days_past),
                    now + timedelta(days=days_future)
                )
        else:
            # Incremental sync
            return self.get_incremental_sync_range()
    
    @abstractmethod
    def get_full_sync_range(self) -> Tuple[Optional[datetime], Optional[datetime]]:
        """
        Define the date range for a full initial sync.
        
        Each source should define what "full" means for their data.
        Can return (None, None) to fetch all available data without date filtering.
        
        Returns:
            Tuple of (start_date, end_date) for full sync, or (None, None) for no date filtering
        """
        pass
    
    @abstractmethod
    def get_incremental_sync_range(self) -> Tuple[Optional[datetime], Optional[datetime]]:
        """
        Define the date range for incremental syncs.
        
        Can return (None, None) if using sync tokens instead of date ranges.
        
        Returns:
            Tuple of (start_date, end_date) for incremental sync
        """
        pass
    
    @abstractmethod
    async def fetch_data(self, start_date: Optional[datetime], end_date: Optional[datetime]) -> Dict[str, Any]:
        """
        Fetch data for the given date range.
        
        If dates are None, implementation should use alternative methods (e.g., sync tokens).
        
        Args:
            start_date: Start of date range (can be None for token-based syncs)
            end_date: End of date range (can be None for token-based syncs)
            
        Returns:
            Dict containing fetched data and sync statistics
        """
        pass
    
    async def run(self) -> Dict[str, Any]:
        """
        Main sync execution flow.
        
        Handles initial vs incremental detection, date range calculation,
        and calls the source-specific fetch_data method.
        
        Returns:
            Dict containing sync results and statistics
        """
        stats = {
            "started_at": datetime.utcnow().isoformat(),
            "is_initial_sync": self.is_initial_sync(),
            "sync_type": getattr(self.stream, 'initial_sync_type', 'limited') if self.is_initial_sync() else 'incremental'
        }
        
        try:
            # Calculate date range based on sync type
            start_date, end_date = self.get_sync_date_range()
            
            # Log sync type and range
            if self.is_initial_sync():
                print(f"Performing initial {stats['sync_type']} sync")
                if start_date and end_date:
                    print(f"Date range: {start_date.isoformat()} to {end_date.isoformat()}")
            else:
                print(f"Performing incremental sync")
                if start_date and end_date:
                    print(f"Date range: {start_date.isoformat() if start_date else 'last sync'} to {end_date.isoformat()}")
                elif not start_date and not end_date:
                    print("Using sync token for incremental sync")
            
            # Fetch data using source-specific implementation
            result = await self.fetch_data(start_date, end_date)
            
            # Update stats with results
            stats.update(result)
            stats["status"] = "success"
            stats["completed_at"] = datetime.utcnow().isoformat()
            
        except Exception as e:
            stats["status"] = "error"
            stats["error"] = str(e)
            stats["completed_at"] = datetime.utcnow().isoformat()
            raise
        
        return stats