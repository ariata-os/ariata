"""
Auto-generated Pydantic model for strava_activities stream.

Generated from: sources/strava/activities/_stream.yaml
Generated at: 2025-08-16T18:38:21.837203
"""

from pydantic import BaseModel, Field
from datetime import datetime, date, time
from typing import Optional, Dict, List, Any
from uuid import UUID


class StreamStravaActivities(BaseModel):
    """Fitness activities and workouts from Strava"""

    # Required fields
    activity_id: int = Field(..., description="Strava activity ID")
    start_date: datetime = Field(..., description="Activity start time (UTC)")

    # Optional fields
    external_id: Optional[str] = Field(None, description="External ID from device/app", max_length=200)
    name: Optional[str] = Field(None, description="Activity name/title", max_length=500)
    type: Optional[str] = Field(None, description="Activity type (Run, Ride, Swim, etc.)", max_length=50)
    sport_type: Optional[str] = Field(None, description="Specific sport type", max_length=50)
    workout_type: Optional[int] = Field(None, description="Workout type code")
    distance: Optional[float] = Field(None, description="Distance in meters")
    moving_time: Optional[int] = Field(None, description="Moving time in seconds")
    elapsed_time: Optional[int] = Field(None, description="Total elapsed time in seconds")
    total_elevation_gain: Optional[float] = Field(None, description="Total elevation gain in meters")
    elev_high: Optional[float] = Field(None, description="Highest elevation in meters")
    elev_low: Optional[float] = Field(None, description="Lowest elevation in meters")
    average_speed: Optional[float] = Field(None, description="Average speed in meters per second")
    max_speed: Optional[float] = Field(None, description="Maximum speed in meters per second")
    average_heartrate: Optional[float] = Field(None, description="Average heart rate in bpm")
    max_heartrate: Optional[float] = Field(None, description="Maximum heart rate in bpm")
    average_cadence: Optional[float] = Field(None, description="Average cadence")
    average_watts: Optional[float] = Field(None, description="Average power in watts")
    kilojoules: Optional[float] = Field(None, description="Total work in kilojoules")
    start_date_local: Optional[datetime] = Field(None, description="Activity start time (local)")
    timezone: Optional[str] = Field(None, description="Timezone of the activity", max_length=50)
    achievement_count: Optional[int] = Field(None, description="Number of achievements")
    kudos_count: Optional[int] = Field(None, description="Number of kudos received")
    comment_count: Optional[int] = Field(None, description="Number of comments")
    start_latlng: Optional[Any] = Field(None, description="Starting coordinates [lat, lng]")
    end_latlng: Optional[Any] = Field(None, description="Ending coordinates [lat, lng]")
    map: Optional[Any] = Field(None, description="Map polyline and summary")
    splits_metric: Optional[Any] = Field(None, description="Kilometer splits")
    splits_standard: Optional[Any] = Field(None, description="Mile splits")
    segment_efforts: Optional[Any] = Field(None, description="Segment efforts within activity")
    gear: Optional[Any] = Field(None, description="Equipment used")
    photos: Optional[Any] = Field(None, description="Activity photos metadata")
    stats: Optional[Any] = Field(None, description="Additional statistics")
    full_activity: Optional[Any] = Field(None, description="Complete activity object for unmapped fields")

    class Config:
        """Model configuration."""
        table_name = "stream_strava_activities"
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