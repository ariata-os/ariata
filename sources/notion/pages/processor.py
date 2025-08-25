"""
Notion Pages Stream Processor - Hybrid Storage
==============================================

This processor handles Notion page data using the stream storage strategy:
- Page metadata → PostgreSQL
- Large content (blocks, attachments) → MinIO
"""

from typing import Dict, Any, List, Type
from datetime import datetime
from pydantic import BaseModel
from sources.base.processors.base import StreamProcessor
from .models import StreamNotionPages


class NotionPagesStreamProcessor(StreamProcessor):
    """
    Process Notion pages streams with hybrid storage.
    
    Page blocks and attachments go to MinIO, metadata goes to PostgreSQL.
    Configuration including minio_fields is auto-loaded from _stream.yaml.
    """
    
    @property
    def model_class(self) -> Type[BaseModel]:
        """Return the Pydantic model for Notion pages data."""
        return StreamNotionPages
    
    async def process_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process Notion page record for hybrid storage.
        
        Splits page content from metadata:
        - blocks, attachments → MinIO
        - All other fields → PostgreSQL
        
        Args:
            record: Raw Notion page with structure:
                {
                    "id": "page_123",
                    "object": "page",
                    "created_time": "2025-01-01T12:00:00.000Z",
                    "last_edited_time": "2025-01-01T13:00:00.000Z",
                    "created_by": {...},
                    "last_edited_by": {...},
                    "cover": {...},
                    "icon": {...},
                    "parent": {...},
                    "archived": false,
                    "properties": {
                        "title": {...},
                        ...
                    },
                    "url": "https://www.notion.so/...",
                    "blocks": [...],  # Large array of block objects
                    "attachments": [...],  # Binary attachments
                    ...
                }
        
        Returns:
            Processed record ready for hybrid storage
        """
        # Parse timestamps
        created_time = self._parse_timestamp(record.get('created_time'))
        last_edited_time = self._parse_timestamp(record.get('last_edited_time'))
        
        # Extract title from properties
        title = self._extract_title(record.get('properties', {}))
        
        # Build processed record for PostgreSQL
        processed = {
            'page_id': record.get('id'),
            'object_type': record.get('object', 'page'),
            'title': title,
            'created_time': created_time,
            'last_edited_time': last_edited_time,
            'archived': record.get('archived', False),
            'url': record.get('url'),
            'public_url': record.get('public_url'),
        }
        
        # Add timestamp for time-series queries
        processed['timestamp'] = last_edited_time or created_time
        
        # Store user information
        if record.get('created_by'):
            processed['created_by'] = record['created_by']
        if record.get('last_edited_by'):
            processed['last_edited_by'] = record['last_edited_by']
        
        # Store page hierarchy
        if record.get('parent'):
            parent = record['parent']
            processed['parent_type'] = parent.get('type')
            if parent.get('type') == 'database_id':
                processed['parent_database_id'] = parent.get('database_id')
            elif parent.get('type') == 'page_id':
                processed['parent_page_id'] = parent.get('page_id')
            elif parent.get('type') == 'workspace':
                processed['parent_workspace'] = True
            processed['parent'] = parent  # Store full parent object
        
        # Store visual elements
        if record.get('icon'):
            processed['icon'] = record['icon']
        if record.get('cover'):
            processed['cover'] = record['cover']
        
        # Store properties (database properties if from a database)
        if record.get('properties'):
            processed['properties'] = record['properties']
            # Extract some common properties for easier querying
            processed['tags'] = self._extract_tags(record['properties'])
            processed['status'] = self._extract_status(record['properties'])
            processed['priority'] = self._extract_priority(record['properties'])
        
        # These fields will go to MinIO based on minio_fields list
        if 'blocks' in record:
            processed['blocks'] = record['blocks']
            # Store block count for reference
            processed['block_count'] = len(record['blocks']) if isinstance(record['blocks'], list) else 0
        
        if 'attachments' in record:
            processed['attachments'] = record['attachments']
            # Store attachment count for reference
            processed['attachment_count'] = len(record['attachments']) if isinstance(record['attachments'], list) else 0
        
        # Store page type metadata
        processed['has_children'] = record.get('has_children', False)
        processed['is_template'] = record.get('is_template', False)
        
        return processed
    
    def _extract_title(self, properties: Dict[str, Any]) -> str:
        """Extract title from Notion properties."""
        # Look for title property (usually called "title" or "Name")
        for prop_name, prop_value in properties.items():
            if prop_value.get('type') == 'title':
                title_array = prop_value.get('title', [])
                if title_array and len(title_array) > 0:
                    return title_array[0].get('plain_text', '')
        return ''
    
    def _extract_tags(self, properties: Dict[str, Any]) -> List[str]:
        """Extract tags from Notion properties."""
        tags = []
        for prop_name, prop_value in properties.items():
            if prop_value.get('type') == 'multi_select':
                multi_select = prop_value.get('multi_select', [])
                tags.extend([item.get('name', '') for item in multi_select])
        return tags
    
    def _extract_status(self, properties: Dict[str, Any]) -> str:
        """Extract status from Notion properties."""
        for prop_name, prop_value in properties.items():
            if prop_value.get('type') == 'status':
                status = prop_value.get('status', {})
                return status.get('name', '')
            elif prop_name.lower() == 'status' and prop_value.get('type') == 'select':
                select = prop_value.get('select', {})
                return select.get('name', '')
        return ''
    
    def _extract_priority(self, properties: Dict[str, Any]) -> str:
        """Extract priority from Notion properties."""
        for prop_name, prop_value in properties.items():
            if prop_name.lower() == 'priority' and prop_value.get('type') == 'select':
                select = prop_value.get('select', {})
                return select.get('name', '')
        return ''
    
    def _parse_timestamp(self, timestamp_str: str) -> datetime:
        """Parse Notion timestamp format."""
        if not timestamp_str:
            return None
        
        from datetime import datetime
        
        try:
            # Notion uses ISO format with milliseconds
            # Example: "2025-01-01T12:00:00.000Z"
            if timestamp_str.endswith('Z'):
                # Remove milliseconds if present
                if '.' in timestamp_str:
                    timestamp_str = timestamp_str.split('.')[0] + 'Z'
                return datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
            else:
                return datetime.fromisoformat(timestamp_str)
        except:
            return None