"""
iOS Microphone Stream Processor - Hybrid Storage Version
========================================================

This processor handles iOS microphone data using the hybrid storage strategy:
- Audio binary data → MinIO
- Metadata and transcriptions → PostgreSQL
"""

from typing import Dict, Any, List, Type
from datetime import datetime
from pydantic import BaseModel
from sources.base.processors.base import StreamProcessor
from .models import StreamIosMic


class IosMicStreamProcessor(StreamProcessor):
    """
    Process iOS microphone streams with hybrid storage.
    
    Audio files go to MinIO, metadata goes to PostgreSQL.
    Configuration including minio_fields is auto-loaded from _stream.yaml.
    """
    
    @property
    def model_class(self) -> Type[BaseModel]:
        """Return the Pydantic model for iOS microphone data."""
        return StreamIosMic
    
    async def process_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process iOS mic record for hybrid storage.
        
        Splits audio data from metadata:
        - audio_data → MinIO (as .wav file)
        - All other fields → PostgreSQL
        
        Args:
            record: Raw iOS mic record with structure:
                {
                    "id": "recording_id",
                    "audio_data": "base64_encoded_audio",
                    "timestamp_start": "2025-01-01T12:00:00Z",
                    "timestamp_end": "2025-01-01T12:00:30Z",
                    "duration": 30000,
                    "audio_level": -45.5,
                    "peak_level": -42.0,
                    "audio_format": "wav",
                    "sample_rate": 44100,
                    ...
                }
        
        Returns:
            Processed record ready for hybrid storage
        """
        # Extract fields for PostgreSQL
        processed = {
            'recording_id': record.get('id'),
            'timestamp': self._parse_timestamp(record.get('timestamp_start')),
            'timestamp_start': self._parse_timestamp(record.get('timestamp_start')),
            'timestamp_end': self._parse_timestamp(record.get('timestamp_end')),
            'duration': record.get('duration'),
            'overlap_duration': record.get('overlap_duration'),
            'audio_format': record.get('audio_format', 'wav'),
            'sample_rate': record.get('sample_rate'),
            'audio_level': record.get('audio_level'),
            'peak_level': record.get('peak_level'),
        }
        
        # Add transcription if available
        if 'transcription' in record:
            processed['transcription_text'] = record['transcription'].get('text')
            processed['transcription_confidence'] = record['transcription'].get('confidence')
            processed['language'] = record['transcription'].get('language', 'en')
        
        # Audio data field - will be handled by base processor's MinIO logic
        if 'audio_data' in record:
            processed['audio_data'] = record['audio_data']
        
        return processed
    
    def _parse_timestamp(self, timestamp_str: str) -> datetime:
        """Parse ISO format timestamp string."""
        if not timestamp_str:
            return None
        
        from datetime import datetime
        
        # Handle different formats
        formats = [
            '%Y-%m-%dT%H:%M:%SZ',
            '%Y-%m-%dT%H:%M:%S.%fZ',
            '%Y-%m-%d %H:%M:%S',
        ]
        
        for fmt in formats:
            try:
                return datetime.strptime(timestamp_str, fmt)
            except ValueError:
                continue
        
        # If all formats fail, try iso format
        try:
            return datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
        except:
            return None


def example_usage():
    """
    Example of how to use the hybrid processor.
    """
    import asyncio
    from sqlalchemy.orm import Session
    
    async def process_mic_data():
        # Sample data from iOS
        ios_data = {
            "device_id": "iphone_123",
            "stream_name": "ios_mic",
            "data": [
                {
                    "id": "rec_001",
                    "audio_data": "UklGRlYAAABXQVZFZm10IBAAAAABAAEA...",  # Base64 audio
                    "timestamp_start": "2025-01-01T12:00:00Z",
                    "timestamp_end": "2025-01-01T12:00:30Z",
                    "duration": 30000,
                    "audio_level": -45.5,
                    "peak_level": -42.0,
                    "audio_format": "wav",
                    "sample_rate": 44100,
                    "transcription": {
                        "text": "Hello, this is a test recording",
                        "confidence": 0.95,
                        "language": "en"
                    }
                }
            ]
        }
        
        # Process with hybrid storage
        processor = IosMicStreamProcessor()
        
        # This would normally use a real database session
        # db_session = Session()
        
        # result = await processor.process_batch(
        #     records=ios_data['data'],
        #     db_session=db_session,
        #     stream_id="stream_instance_123"
        # )
        
        # Result would show:
        # {
        #     'processed': 1,
        #     'table': 'stream_ios_mic',
        #     'postgres_records': 1,
        #     'minio_assets': 1  # audio_data went to MinIO
        # }
    
    # Run example
    # asyncio.run(process_mic_data())