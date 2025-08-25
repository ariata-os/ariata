"""
Auto-generated Pydantic model for ios_healthkit stream.

Generated from: sources/ios/healthkit/_stream.yaml
Generated at: 2025-08-16T18:38:21.790171
"""

from pydantic import BaseModel, Field
from datetime import datetime, date, time
from typing import Optional, Dict, List, Any
from uuid import UUID


class StreamIosHealthkit(BaseModel):
    """Health metrics from iOS HealthKit including heart rate, HRV, steps, and activity"""

    # Optional fields
    heart_rate: Optional[float] = Field(None, description="Heart rate in beats per minute")
    hrv: Optional[float] = Field(None, description="Heart rate variability in milliseconds")
    activity_type: Optional[str] = Field(None, description="Type of activity (sleeping, walking, running, etc.)", max_length=50)
    confidence: Optional[float] = Field(None, description="Confidence level of the measurement")
    steps: Optional[int] = Field(None, description="Number of steps")
    active_energy: Optional[float] = Field(None, description="Active energy burned in kcal")
    sleep_stage: Optional[str] = Field(None, description="Sleep stage (awake, light, deep, rem)", max_length=20)
    workout_type: Optional[str] = Field(None, description="Type of workout activity", max_length=50)
    workout_duration: Optional[int] = Field(None, description="Workout duration in seconds")
    device_name: Optional[str] = Field(None, description="Name of the device that recorded the data", max_length=100)
    raw_data: Optional[Any] = Field(None, description="Additional fields not mapped to columns")

    class Config:
        """Model configuration."""
        table_name = "stream_ios_healthkit"
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