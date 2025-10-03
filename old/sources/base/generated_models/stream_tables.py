"""
Auto-generated SQLAlchemy models from YAML stream schemas.
Generated at: 2025-08-17T17:47:55.369190Z
"""

from uuid import uuid4
from datetime import datetime, date, time
from decimal import Decimal
from typing import Dict, Any, List, Optional
from sqlalchemy import Column, String, Integer, BigInteger, SmallInteger, Float, Numeric
from sqlalchemy import Boolean, DateTime, Date, Time, Text, JSON, LargeBinary
from sqlalchemy import ForeignKey, Index, UUID, ARRAY
from sqlalchemy.ext.declarative import declarative_base

Base = declarative_base()

class StreamGoogleCalendar(Base):
    """
    Calendar events and appointments from Google Calendar
    """
    __tablename__ = "stream_google_calendar"

    event_id = Column(String(200), nullable=False)  # Google Calendar event ID
    calendar_id = Column(String(200), nullable=False)  # Calendar ID (email or calendar identifier)
    ical_uid = Column(String(200))  # iCalendar UID for the event
    summary = Column(String(500))  # Event title/summary
    description = Column(Text)  # Event description
    location = Column(String(500))  # Event location
    status = Column(String(20))  # Event status (confirmed, tentative, cancelled)
    start_time = Column(DateTime(timezone=True), nullable=False)  # Event start time
    end_time = Column(DateTime(timezone=True), nullable=False)  # Event end time
    all_day = Column(Boolean, default=False)  # Whether this is an all-day event
    timezone = Column(String(50))  # Event timezone
    html_link = Column(String(500))  # Link to event in Google Calendar
    created_time = Column(DateTime(timezone=True))  # When the event was created
    updated_time = Column(DateTime(timezone=True))  # When the event was last updated
    event_type = Column(String(50))  # Type of event (default, outOfOffice, focusTime, etc.)
    creator = Column(JSON)  # Event creator information
    organizer = Column(JSON)  # Event organizer information
    attendees = Column(JSON)  # List of attendees with response status
    reminders = Column(JSON)  # Reminder settings
    recurrence = Column(JSON)  # Recurrence rules (RRULE)
    conference_data = Column(JSON)  # Video/phone conference details
    full_event = Column(JSON)  # Complete event object for unmapped fields

    # Indexes
    __table_args__ = (Index("idx_stream_google_calendar_timestamp", "timestamp"),)
    __table_args__ = (Index("idx_stream_google_calendar_start_time", "start_time"),)
    __table_args__ = (Index("idx_stream_google_calendar_event_id", "event_id"),)


class StreamIosHealthkit(Base):
    """
    Health metrics from iOS HealthKit including heart rate, HRV, steps, and activity
    """
    __tablename__ = "stream_ios_healthkit"

    heart_rate = Column(Float)  # Heart rate in beats per minute
    hrv = Column(Float)  # Heart rate variability in milliseconds
    activity_type = Column(String(50))  # Type of activity (sleeping, walking, running, etc.)
    confidence = Column(Float)  # Confidence level of the measurement
    steps = Column(Integer)  # Number of steps
    active_energy = Column(Float)  # Active energy burned in kcal
    sleep_stage = Column(String(20))  # Sleep stage (awake, light, deep, rem)
    workout_type = Column(String(50))  # Type of workout activity
    workout_duration = Column(Integer)  # Workout duration in seconds
    device_name = Column(String(100))  # Name of the device that recorded the data
    raw_data = Column(JSON)  # Additional fields not mapped to columns

    # Indexes
    __table_args__ = (Index("idx_stream_ios_healthkit_timestamp", "timestamp"),)


class StreamIosLocation(Base):
    """
    GPS and location data from iOS Core Location
    """
    __tablename__ = "stream_ios_location"

    latitude = Column(Float, nullable=False)  # Latitude coordinate
    longitude = Column(Float, nullable=False)  # Longitude coordinate
    altitude = Column(Float)  # Altitude in meters
    horizontal_accuracy = Column(Float)  # Horizontal accuracy in meters
    vertical_accuracy = Column(Float)  # Vertical accuracy in meters
    speed = Column(Float)  # Speed in meters per second
    course = Column(Float)  # Course/heading in degrees from true north
    floor = Column(Integer)  # Floor level in building
    activity_type = Column(String(50))  # Type of activity (stationary, walking, running, automotive, etc.)
    address = Column(String(500))  # Reverse geocoded address
    place_name = Column(String(200))  # Name of the place/venue
    raw_data = Column(JSON)  # Additional location metadata

    # Indexes
    __table_args__ = (Index("idx_stream_ios_location_timestamp", "timestamp"),)


class StreamIosMic(Base):
    """
    Audio metadata and transcriptions from iOS microphone
    """
    __tablename__ = "stream_ios_mic"

    recording_id = Column(String(100), nullable=False)  # Unique identifier for the recording
    timestamp_start = Column(DateTime(timezone=True), nullable=False)  # Start time of the recording
    timestamp_end = Column(DateTime(timezone=True), nullable=False)  # End time of the recording
    duration = Column(Integer, nullable=False)  # Duration in milliseconds
    overlap_duration = Column(Float)  # Overlap duration with previous recording in seconds
    audio_format = Column(String(10))  # Audio format (wav, mp3, etc.)
    sample_rate = Column(Integer)  # Sample rate in Hz
    audio_level = Column(Float)  # Average audio level in dB
    peak_level = Column(Float)  # Peak audio level in dB
    transcription_text = Column(Text)  # Transcribed text from audio
    transcription_confidence = Column(Float)  # Confidence score of transcription
    language = Column(String(10))  # Detected language code
    minio_path = Column(String(500))  # Path to audio file in MinIO storage
    file_size = Column(Integer)  # Size of audio file in bytes
    raw_data = Column(JSON)  # Additional metadata

    # Indexes
    __table_args__ = (Index("idx_stream_ios_mic_timestamp", "timestamp"),)
    __table_args__ = (Index("idx_stream_ios_mic_timestamp_start", "timestamp_start"),)


class StreamMacApps(Base):
    """
    Application focus events from macOS
    """
    __tablename__ = "stream_mac_apps"

    app_name = Column(String(200), nullable=False)  # Application name
    bundle_id = Column(String(200))  # macOS bundle identifier
    event_type = Column(String(50), nullable=False)  # Event type (focus_gained, focus_lost, launch, quit)

    # Indexes
    __table_args__ = (Index("idx_stream_mac_apps_timestamp", "timestamp"),)
    __table_args__ = (Index("idx_stream_mac_apps_app_name_timestamp", "app_name", "timestamp"),)
    __table_args__ = (Index("idx_stream_mac_apps_event_type_timestamp", "event_type", "timestamp"),)


class StreamNotionPages(Base):
    """
    Pages and databases from Notion workspace
    """
    __tablename__ = "stream_notion_pages"

    page_id = Column(String(100), nullable=False)  # Notion page UUID
    parent_id = Column(String(100))  # Parent page or workspace ID
    parent_type = Column(String(20))  # Type of parent (page, database, workspace)
    title = Column(String(500))  # Page title
    object_type = Column(String(20))  # Object type (page, database)
    archived = Column(Boolean, default=False)  # Whether the page is archived
    url = Column(String(500))  # Public URL if shared
    created_time = Column(DateTime(timezone=True))  # When the page was created in Notion
    created_by = Column(String(100))  # User ID who created the page
    last_edited_time = Column(DateTime(timezone=True))  # When the page was last edited in Notion
    last_edited_by = Column(String(100))  # User ID who last edited the page
    content_text = Column(Text)  # Extracted plain text content
    content_markdown = Column(Text)  # Content converted to Markdown
    properties = Column(JSON)  # Database properties and values
    icon = Column(JSON)  # Page icon (emoji or image)
    cover = Column(JSON)  # Page cover image
    parent = Column(JSON)  # Full parent relationship data
    blocks = Column(JSON)  # Page content blocks (if small)
    minio_path = Column(String(500))  # Path to full content in MinIO (if large)
    full_page = Column(JSON)  # Complete page object for unmapped fields

    # Indexes
    __table_args__ = (Index("idx_stream_notion_pages_timestamp", "timestamp"),)
    __table_args__ = (Index("idx_stream_notion_pages_last_edited_time", "last_edited_time"),)
    __table_args__ = (Index("idx_stream_notion_pages_page_id", "page_id"),)


class StreamStravaActivities(Base):
    """
    Fitness activities and workouts from Strava
    """
    __tablename__ = "stream_strava_activities"

    activity_id = Column(BigInteger, nullable=False)  # Strava activity ID
    external_id = Column(String(200))  # External ID from device/app
    name = Column(String(500))  # Activity name/title
    type = Column(String(50))  # Activity type (Run, Ride, Swim, etc.)
    sport_type = Column(String(50))  # Specific sport type
    workout_type = Column(Integer)  # Workout type code
    distance = Column(Float)  # Distance in meters
    moving_time = Column(Integer)  # Moving time in seconds
    elapsed_time = Column(Integer)  # Total elapsed time in seconds
    total_elevation_gain = Column(Float)  # Total elevation gain in meters
    elev_high = Column(Float)  # Highest elevation in meters
    elev_low = Column(Float)  # Lowest elevation in meters
    average_speed = Column(Float)  # Average speed in meters per second
    max_speed = Column(Float)  # Maximum speed in meters per second
    average_heartrate = Column(Float)  # Average heart rate in bpm
    max_heartrate = Column(Float)  # Maximum heart rate in bpm
    average_cadence = Column(Float)  # Average cadence
    average_watts = Column(Float)  # Average power in watts
    kilojoules = Column(Float)  # Total work in kilojoules
    start_date = Column(DateTime(timezone=True), nullable=False)  # Activity start time (UTC)
    start_date_local = Column(DateTime(timezone=True))  # Activity start time (local)
    timezone = Column(String(50))  # Timezone of the activity
    achievement_count = Column(Integer)  # Number of achievements
    kudos_count = Column(Integer)  # Number of kudos received
    comment_count = Column(Integer)  # Number of comments
    start_latlng = Column(JSON)  # Starting coordinates [lat, lng]
    end_latlng = Column(JSON)  # Ending coordinates [lat, lng]
    map = Column(JSON)  # Map polyline and summary
    splits_metric = Column(JSON)  # Kilometer splits
    splits_standard = Column(JSON)  # Mile splits
    segment_efforts = Column(JSON)  # Segment efforts within activity
    gear = Column(JSON)  # Equipment used
    photos = Column(JSON)  # Activity photos metadata
    stats = Column(JSON)  # Additional statistics
    full_activity = Column(JSON)  # Complete activity object for unmapped fields

    # Indexes
    __table_args__ = (Index("idx_stream_strava_activities_timestamp", "timestamp"),)
    __table_args__ = (Index("idx_stream_strava_activities_start_date", "start_date"),)
    __table_args__ = (Index("idx_stream_strava_activities_activity_id", "activity_id"),)

