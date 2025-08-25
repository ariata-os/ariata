"""Storage utility helpers for common data operations."""

from typing import Any, Dict, Optional
from datetime import datetime, timezone
from uuid import uuid4
import json

from sources.base.storage.minio import store_raw_data as minio_store_raw_data


class StorageHelper:
    """Unified storage utilities for all sources."""
    
    @staticmethod
    async def store_stream_data(
        stream_name: str,
        stream_id: str,
        data: Any,
        timestamp: Optional[datetime] = None
    ) -> str:
        """
        Store stream data to MinIO with consistent formatting.
        
        Args:
            stream_name: Name of the stream (e.g., 'google_calendar')
            stream_id: Stream instance ID
            data: Data to store (will be JSON serialized)
            timestamp: Optional timestamp (defaults to current UTC time)
            
        Returns:
            Object name/key of stored data
        """
        if timestamp is None:
            timestamp = datetime.now(timezone.utc)
        
        # Use the existing MinIO store function with correct parameters
        return await minio_store_raw_data(
            stream_name=stream_name,
            connection_id=str(stream_id),
            data=data,
            timestamp=timestamp
        )
    
    @staticmethod
    async def store_batch_data(
        stream_name: str,
        stream_id: str,
        items: list,
        metadata: Optional[Dict[str, Any]] = None
    ) -> str:
        """
        Store a batch of items as a single unit.
        
        Args:
            stream_name: Name of the stream
            stream_id: Stream instance ID
            items: List of items to store
            metadata: Optional metadata about the batch
            
        Returns:
            Object name/key of stored batch
        """
        batch_data = {
            "batch_id": str(uuid4()),
            "item_count": len(items),
            "items": items,
            "timestamp": datetime.now(timezone.utc).isoformat()
        }
        
        if metadata:
            batch_data["metadata"] = metadata
        
        return await StorageHelper.store_stream_data(
            stream_name=stream_name,
            stream_id=stream_id,
            data=batch_data
        )
    
    @staticmethod
    async def store_sync_result(
        stream_name: str,
        stream_id: str,
        sync_stats: Dict[str, Any],
        data: Any
    ) -> str:
        """
        Store sync results with standardized metadata.
        
        Args:
            stream_name: Name of the stream
            stream_id: Stream instance ID
            sync_stats: Statistics from the sync operation
            data: The actual synced data
            
        Returns:
            Object name/key of stored data
        """
        wrapped_data = {
            "_sync_metadata": {
                "synced_at": datetime.now(timezone.utc).isoformat(),
                "stream_name": stream_name,
                "stream_id": str(stream_id),
                "stats": sync_stats
            },
            "data": data
        }
        
        return await StorageHelper.store_stream_data(
            stream_name=stream_name,
            stream_id=stream_id,
            data=wrapped_data
        )
    
    @staticmethod
    def format_metadata(
        source_name: str,
        stream_name: str,
        additional_metadata: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Create standardized metadata for storage operations.
        
        Args:
            source_name: Name of the source (e.g., 'google')
            stream_name: Name of the stream (e.g., 'calendar')
            additional_metadata: Additional metadata to include
            
        Returns:
            Formatted metadata dictionary
        """
        metadata = {
            "source": source_name,
            "stream": stream_name,
            "stored_at": datetime.now(timezone.utc).isoformat(),
            "storage_version": "2.0"
        }
        
        if additional_metadata:
            metadata.update(additional_metadata)
        
        return metadata
    
    @staticmethod
    async def store_error(
        stream_name: str,
        stream_id: str,
        error: Exception,
        context: Optional[Dict[str, Any]] = None
    ) -> str:
        """
        Store error information for debugging.
        
        Args:
            stream_name: Name of the stream
            stream_id: Stream instance ID
            error: The exception that occurred
            context: Additional context about when the error occurred
            
        Returns:
            Object name/key of stored error data
        """
        error_data = {
            "error_type": type(error).__name__,
            "error_message": str(error),
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "stream_name": stream_name,
            "stream_id": str(stream_id)
        }
        
        if context:
            error_data["context"] = context
        
        # Include traceback if available
        import traceback
        if hasattr(error, '__traceback__'):
            error_data["traceback"] = traceback.format_tb(error.__traceback__)
        
        return await StorageHelper.store_stream_data(
            stream_name=f"{stream_name}_errors",
            stream_id=stream_id,
            data=error_data
        )