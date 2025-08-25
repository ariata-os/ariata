"""
iOS Location Stream Processor - PostgreSQL Only
================================================

Location data is all structured (coordinates, speed, altitude) so everything
goes to PostgreSQL. No MinIO storage needed.
"""

from typing import Dict, Any, List, Type
from datetime import datetime
from pydantic import BaseModel
from sources.base.processors.base import StreamProcessor
from .models import StreamIosLocation


class IosLocationStreamProcessor(StreamProcessor):
    """
    Process iOS location streams - all data goes to PostgreSQL.
    
    No binary data in location streams, so MinIO is not used.
    Configuration is auto-loaded from _stream.yaml.
    """
    
    @property
    def model_class(self) -> Type[BaseModel]:
        """Return the Pydantic model for iOS location data."""
        return StreamIosLocation
    
    async def process_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process iOS location record for PostgreSQL storage.
        
        All fields go directly to PostgreSQL - no MinIO needed.
        
        Args:
            record: Raw iOS location record with structure:
                {
                    "timestamp": "2025-01-01T12:00:00Z",
                    "latitude": 36.174,
                    "longitude": -86.744,
                    "altitude": 164.97,
                    "horizontal_accuracy": 5.0,
                    "vertical_accuracy": 3.0,
                    "speed": 0.0,
                    "course": 0.0,
                    "floor": 2,
                    "activity_type": "walking"
                }
        
        Returns:
            Processed record ready for PostgreSQL storage
        """
        # All fields go to PostgreSQL
        processed = {
            'timestamp': self._parse_timestamp(record.get('timestamp')),
            'latitude': record.get('latitude'),
            'longitude': record.get('longitude'),
            'altitude': record.get('altitude'),
            'horizontal_accuracy': record.get('horizontal_accuracy'),
            'vertical_accuracy': record.get('vertical_accuracy'),
            'speed': record.get('speed'),
            'course': record.get('course'),
            'floor': record.get('floor'),
            'activity_type': record.get('activity_type'),
        }
        
        # Optional: Add reverse geocoded address if available
        if 'address' in record:
            processed['address'] = record['address']
        if 'place_name' in record:
            processed['place_name'] = record['place_name']
        
        # Store any additional metadata in raw_data JSONB column
        extra_fields = set(record.keys()) - set(processed.keys())
        if extra_fields:
            processed['raw_data'] = {k: record[k] for k in extra_fields}
        
        return processed
    
    def _parse_timestamp(self, timestamp_str: str) -> datetime:
        """Parse ISO format timestamp string."""
        if not timestamp_str:
            return None
        
        from datetime import datetime
        
        try:
            return datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
        except:
            return None