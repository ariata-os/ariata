"""
Auto-generated Pydantic model for google_calendar stream.

Generated from: sources/google/calendar/_stream.yaml
Generated at: 2025-08-16T18:38:21.783405
"""

from pydantic import BaseModel, Field
from datetime import datetime, date, time
from typing import Optional, Dict, List, Any
from uuid import UUID


class StreamGoogleCalendar(BaseModel):
    """Calendar events and appointments from Google Calendar"""

    # Required fields
    event_id: str = Field(..., description="Google Calendar event ID", max_length=200)
    calendar_id: str = Field(..., description="Calendar ID (email or calendar identifier)", max_length=200)
    start_time: datetime = Field(..., description="Event start time")
    end_time: datetime = Field(..., description="Event end time")

    # Optional fields
    ical_uid: Optional[str] = Field(None, description="iCalendar UID for the event", max_length=200)
    summary: Optional[str] = Field(None, description="Event title/summary", max_length=500)
    description: Optional[str] = Field(None, description="Event description")
    location: Optional[str] = Field(None, description="Event location", max_length=500)
    status: Optional[str] = Field(None, description="Event status (confirmed, tentative, cancelled)", max_length=20)
    all_day: Optional[bool] = Field(None, description="Whether this is an all-day event")
    timezone: Optional[str] = Field(None, description="Event timezone", max_length=50)
    html_link: Optional[str] = Field(None, description="Link to event in Google Calendar", max_length=500)
    created_time: Optional[datetime] = Field(None, description="When the event was created")
    updated_time: Optional[datetime] = Field(None, description="When the event was last updated")
    event_type: Optional[str] = Field(None, description="Type of event (default, outOfOffice, focusTime, etc.)", max_length=50)
    creator: Optional[Any] = Field(None, description="Event creator information")
    organizer: Optional[Any] = Field(None, description="Event organizer information")
    attendees: Optional[Any] = Field(None, description="List of attendees with response status")
    reminders: Optional[Any] = Field(None, description="Reminder settings")
    recurrence: Optional[Any] = Field(None, description="Recurrence rules (RRULE)")
    conference_data: Optional[Any] = Field(None, description="Video/phone conference details")
    full_event: Optional[Any] = Field(None, description="Complete event object for unmapped fields")

    class Config:
        """Model configuration."""
        table_name = "stream_google_calendar"
        storage_strategy = "postgres_only"
        minio_fields = []
        orm_mode = True
        validate_assignment = True

    def dict_for_db(self) -> Dict[str, Any]:
        """
        Get dictionary for database insertion.
        Excludes auto-managed fields and MinIO fields.
        """
        data = self.dict(exclude_unset=True)
        # Remove auto-managed fields
        for field in ["id", "created_at", "updated_at"]:
            data.pop(field, None)
        # Remove MinIO fields (they go to object storage)
        for field in self.Config.minio_fields:
            data.pop(field, None)
        return data