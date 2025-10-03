"""
Google Calendar Stream Processor - PostgreSQL Storage
=====================================================

This processor handles Google Calendar events using the stream storage strategy:
- All event data → PostgreSQL
- No MinIO needed (all data is structured)
"""

from typing import Dict, Any, List, Type
from datetime import datetime
from pydantic import BaseModel
from sources.base.processors.base import StreamProcessor
from .models import StreamGoogleCalendar


class GoogleCalendarStreamProcessor(StreamProcessor):
    """
    Process Google Calendar streams - all data goes to PostgreSQL.
    
    Calendar events are all structured data, so no MinIO storage needed.
    Configuration is auto-loaded from _stream.yaml.
    """
    
    @property
    def model_class(self) -> Type[BaseModel]:
        """Return the Pydantic model for Google Calendar."""
        return StreamGoogleCalendar
    
    async def process_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process Google Calendar event record for PostgreSQL storage.
        
        All fields go directly to PostgreSQL - no MinIO needed.
        
        Args:
            record: Raw Google Calendar event with structure:
                {
                    "event": {
                        "id": "event_123",
                        "summary": "Meeting",
                        "description": "Team sync",
                        "location": "Conference Room A",
                        "start": {"dateTime": "2025-01-01T10:00:00Z"},
                        "end": {"dateTime": "2025-01-01T11:00:00Z"},
                        "status": "confirmed",
                        "htmlLink": "https://calendar.google.com/...",
                        "creator": {...},
                        "organizer": {...},
                        "attendees": [...],
                        ...
                    },
                    "calendar": {
                        "id": "calendar_123",
                        "summary": "Work Calendar",
                        ...
                    }
                }
        
        Returns:
            Processed record ready for PostgreSQL storage
        """
        # Extract event and calendar info
        event = record.get('event', {})
        calendar = record.get('calendar', {})
        
        # Parse timestamps
        start_time = self._parse_event_time(event.get('start'))
        end_time = self._parse_event_time(event.get('end'))
        
        # Build processed record - matching database schema
        processed = {
            'event_id': event.get('id'),
            'calendar_id': calendar.get('id'),
            'summary': event.get('summary'),
            'description': event.get('description'),
            'location': event.get('location'),
            'start_time': start_time,
            'end_time': end_time,
            'status': event.get('status'),
            'html_link': event.get('htmlLink'),
            'all_day': self._is_all_day_event(event.get('start')),  # Changed from is_all_day
            'timezone': calendar.get('timeZone'),  # Changed from calendar_timezone
            'event_type': event.get('eventType', 'default'),
        }
        
        # Add timestamp for time-series queries
        processed['timestamp'] = start_time
        
        # Store complex objects as JSONB
        if event.get('creator'):
            processed['creator'] = event['creator']
        if event.get('organizer'):
            processed['organizer'] = event['organizer']
        if event.get('attendees'):
            processed['attendees'] = event['attendees']
        if event.get('reminders'):
            processed['reminders'] = event['reminders']
        if event.get('recurrence'):
            processed['recurrence'] = event['recurrence']
        
        # Store the full event data for reference
        processed['full_event'] = event
        
        return processed
    
    def _parse_event_time(self, time_obj: Dict[str, Any]) -> datetime:
        """Parse Google Calendar time object."""
        if not time_obj:
            return None
        
        # Handle dateTime format (specific time)
        if 'dateTime' in time_obj:
            return self._parse_timestamp(time_obj['dateTime'])
        
        # Handle date format (all-day events)
        if 'date' in time_obj:
            # Parse date and set to midnight
            date_str = time_obj['date']
            return datetime.fromisoformat(date_str + 'T00:00:00+00:00')
        
        return None
    
    def _is_all_day_event(self, time_obj: Dict[str, Any]) -> bool:
        """Check if event is all-day based on time format."""
        if not time_obj:
            return False
        return 'date' in time_obj and 'dateTime' not in time_obj
    
    def _parse_timestamp(self, timestamp_str: str) -> datetime:
        """Parse ISO format timestamp string."""
        if not timestamp_str:
            return None
        
        from datetime import datetime
        
        try:
            # Handle various ISO formats
            if 'T' in timestamp_str:
                # Full timestamp with time
                return datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
            else:
                # Date only - set to midnight
                return datetime.fromisoformat(timestamp_str + 'T00:00:00+00:00')
        except:
            return None