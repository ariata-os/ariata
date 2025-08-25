"""
Auto-generated Pydantic model for ios_location stream.

Generated from: sources/ios/location/_stream.yaml
Generated at: 2025-08-16T18:38:21.796960
"""

from pydantic import BaseModel, Field
from datetime import datetime, date, time
from typing import Optional, Dict, List, Any
from uuid import UUID


class StreamIosLocation(BaseModel):
    """GPS and location data from iOS Core Location"""

    # Required fields
    latitude: float = Field(..., description="Latitude coordinate")
    longitude: float = Field(..., description="Longitude coordinate")

    # Optional fields
    altitude: Optional[float] = Field(None, description="Altitude in meters")
    horizontal_accuracy: Optional[float] = Field(None, description="Horizontal accuracy in meters")
    vertical_accuracy: Optional[float] = Field(None, description="Vertical accuracy in meters")
    speed: Optional[float] = Field(None, description="Speed in meters per second")
    course: Optional[float] = Field(None, description="Course/heading in degrees from true north")
    floor: Optional[int] = Field(None, description="Floor level in building")
    activity_type: Optional[str] = Field(None, description="Type of activity (stationary, walking, running, automotive, etc.)", max_length=50)
    address: Optional[str] = Field(None, description="Reverse geocoded address", max_length=500)
    place_name: Optional[str] = Field(None, description="Name of the place/venue", max_length=200)
    raw_data: Optional[Any] = Field(None, description="Additional location metadata")

    class Config:
        """Model configuration."""
        table_name = "stream_ios_location"
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