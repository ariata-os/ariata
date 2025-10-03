"""
Mac Apps Stream Processor - Pure Data
======================================

Simple passthrough processor for Mac app events.
No semantic processing - just stores raw facts.
"""

from typing import Dict, Any, Type
from datetime import datetime
from pydantic import BaseModel
from sources.base.processors.base import StreamProcessor
from .models import StreamMacApps


class MacAppsStreamProcessor(StreamProcessor):
    """
    Process Mac application focus events.
    
    Simple passthrough processor - stores facts without interpretation.
    """
    
    @property
    def model_class(self) -> Type[BaseModel]:
        """Return the Pydantic model for Mac apps data."""
        return StreamMacApps
    
    async def process_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process Mac app activity record for PostgreSQL storage.
        
        Args:
            record: Raw Mac app event with structure:
                {
                    "app_name": "Visual Studio Code",
                    "bundle_id": "com.microsoft.VSCode",
                    "event_type": "focus_gained",
                    "timestamp": "2025-01-01T12:00:00Z"
                }
        
        Returns:
            Processed record ready for PostgreSQL storage
        """
        # Parse timestamp
        timestamp = self._parse_timestamp(record.get('timestamp'))
        
        # Return the data as-is, no semantic processing
        return {
            'app_name': record.get('app_name'),
            'bundle_id': record.get('bundle_id'),
            'event_type': record.get('event_type'),
            'timestamp': timestamp,
        }
    
    def _parse_timestamp(self, timestamp_str: str) -> datetime:
        """Parse ISO format timestamp string."""
        if not timestamp_str:
            return None
        
        try:
            # Handle ISO format with Z
            if timestamp_str.endswith('Z'):
                return datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
            else:
                return datetime.fromisoformat(timestamp_str)
        except:
            return None