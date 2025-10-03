"""
Mac Messages Stream Processor
==============================

Processes iMessage and SMS data from macOS Messages app.
Handles deduplication based on message_id and incremental sync.
"""

from typing import Dict, Any, Type, Optional, List
from datetime import datetime
from pydantic import BaseModel
from sources.base.processors.base import StreamProcessor
from .models import StreamMacMessages


class MacMessagesStreamProcessor(StreamProcessor):
    """
    Process Mac Messages (iMessage/SMS) events.
    
    Handles:
    - Message deduplication using message_id
    - Attachment metadata extraction
    - Timestamp normalization from macOS epoch
    """
    
    @property
    def model_class(self) -> Type[BaseModel]:
        """Return the Pydantic model for Mac messages data."""
        return StreamMacMessages
    
    async def process_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process Mac message record for PostgreSQL storage.
        
        Args:
            record: Raw message event with structure:
                {
                    "message_id": "GUID-12345",
                    "chat_id": "chat123456",
                    "handle_id": "+1234567890",
                    "text": "Hello world",
                    "service": "iMessage",
                    "is_from_me": true,
                    "date": 1234567890,  # macOS timestamp
                    "date_read": 1234567891,
                    "date_delivered": 1234567891,
                    "is_read": true,
                    "is_delivered": true,
                    "is_sent": true,
                    "cache_has_attachments": false,
                    "attachment_count": 0,
                    "attachment_info": [],
                    "group_title": null,
                    "associated_message_guid": null,
                    "associated_message_type": null,
                    "expressive_send_style_id": null
                }
        
        Returns:
            Processed record ready for PostgreSQL storage
        """
        # Parse timestamps - handle macOS Core Data timestamps
        date = self._parse_macos_timestamp(record.get('date'))
        date_read = self._parse_macos_timestamp(record.get('date_read'))
        date_delivered = self._parse_macos_timestamp(record.get('date_delivered'))
        
        # Process attachment info if present
        attachment_info = record.get('attachment_info')
        if attachment_info and isinstance(attachment_info, list):
            # Ensure attachment info is properly formatted
            attachment_info = [self._process_attachment(att) for att in attachment_info]
        
        # Build processed record
        processed = {
            'message_id': record.get('message_id'),
            'chat_id': record.get('chat_id'),
            'handle_id': record.get('handle_id'),
            'text': record.get('text'),
            'service': record.get('service', 'iMessage'),
            'is_from_me': bool(record.get('is_from_me', False)),
            'date': date,
            'date_read': date_read,
            'date_delivered': date_delivered,
            'is_read': bool(record.get('is_read', False)),
            'is_delivered': bool(record.get('is_delivered', False)),
            'is_sent': bool(record.get('is_sent', False)),
            'cache_has_attachments': bool(record.get('cache_has_attachments', False)),
            'attachment_count': record.get('attachment_count', 0),
            'attachment_info': attachment_info,
            'group_title': record.get('group_title'),
            'associated_message_guid': record.get('associated_message_guid'),
            'associated_message_type': record.get('associated_message_type'),
            'expressive_send_style_id': record.get('expressive_send_style_id'),
            'timestamp': date,  # Use message date as the timestamp
        }
        
        # Store any additional fields in raw_data
        known_fields = set(processed.keys())
        raw_data = {k: v for k, v in record.items() if k not in known_fields}
        if raw_data:
            processed['raw_data'] = raw_data
        
        return processed
    
    def _parse_macos_timestamp(self, timestamp: Optional[Any]) -> Optional[datetime]:
        """
        Parse macOS Core Data timestamp.
        
        macOS Messages uses Core Data timestamps which are seconds since 2001-01-01.
        We need to convert to standard Unix timestamp.
        """
        if timestamp is None:
            return None
        
        try:
            # If already a datetime string
            if isinstance(timestamp, str):
                if timestamp.endswith('Z'):
                    return datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
                else:
                    return datetime.fromisoformat(timestamp)
            
            # If it's a macOS Core Data timestamp (seconds since 2001-01-01)
            # Core Data epoch: 2001-01-01 00:00:00 UTC
            # Unix epoch: 1970-01-01 00:00:00 UTC
            # Difference: 978307200 seconds
            MACOS_EPOCH_OFFSET = 978307200
            
            # Convert to float in case it's a string number
            timestamp_float = float(timestamp)
            
            # If the timestamp is reasonable for Core Data (< 1 billion = before 2032)
            if timestamp_float < 1_000_000_000:
                # It's a Core Data timestamp
                unix_timestamp = timestamp_float + MACOS_EPOCH_OFFSET
            else:
                # It's already a Unix timestamp
                unix_timestamp = timestamp_float
            
            return datetime.fromtimestamp(unix_timestamp)
        except (ValueError, TypeError) as e:
            print(f"Failed to parse timestamp {timestamp}: {e}")
            return None
    
    def _process_attachment(self, attachment: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process attachment metadata.
        
        Extract relevant fields and ensure proper formatting.
        """
        return {
            'filename': attachment.get('filename'),
            'mime_type': attachment.get('mime_type'),
            'file_size': attachment.get('file_size'),
            'transfer_name': attachment.get('transfer_name'),
            'uti': attachment.get('uti'),  # Uniform Type Identifier
        }
    
    async def deduplicate_records(self, records: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """
        Deduplicate messages based on message_id.
        
        Since message_id is unique in iMessage, we can use it for deduplication.
        """
        seen_ids = set()
        unique_records = []
        
        for record in records:
            message_id = record.get('message_id')
            if message_id and message_id not in seen_ids:
                seen_ids.add(message_id)
                unique_records.append(record)
        
        return unique_records