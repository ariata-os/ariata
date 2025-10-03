"""
Auto-generated Pydantic model for mac_apps stream.

Generated from: sources/mac/apps/_stream.yaml
Generated at: 2025-08-16T18:38:21.813839
"""

from pydantic import BaseModel, Field
from datetime import datetime, date, time
from typing import Optional, Dict, List, Any
from uuid import UUID


class StreamMacApps(BaseModel):
    """Application usage and activity from macOS"""

    # Required fields
    app_name: str = Field(..., description="Application name", max_length=200)
    event_type: str = Field(..., description="Event type (focus_gained, focus_lost, launch, quit)", max_length=50)

    # Optional fields
    bundle_id: Optional[str] = Field(None, description="macOS bundle identifier", max_length=200)
    process_name: Optional[str] = Field(None, description="Process name", max_length=200)
    window_title: Optional[str] = Field(None, description="Active window title", max_length=500)
    window_id: Optional[int] = Field(None, description="Window identifier")
    duration: Optional[int] = Field(None, description="Duration of focus session in seconds")
    idle_time: Optional[int] = Field(None, description="Idle time in seconds before event")
    pid: Optional[int] = Field(None, description="Process ID")
    is_active: Optional[bool] = Field(None, description="Whether app is currently active")
    is_hidden: Optional[bool] = Field(None, description="Whether app is hidden")
    is_minimized: Optional[bool] = Field(None, description="Whether app is minimized")
    category: Optional[str] = Field(None, description="App category (productivity, communication, entertainment, etc.)", max_length=50)
    project: Optional[str] = Field(None, description="Associated project (detected from window title)", max_length=200)
    url: Optional[str] = Field(None, description="URL if browser, null otherwise", max_length=1000)
    document_path: Optional[str] = Field(None, description="Path to open document if applicable", max_length=500)
    window_info: Optional[Any] = Field(None, description="Additional window information (size, position, screen)")
    process_info: Optional[Any] = Field(None, description="Process details (CPU, memory, threads)")
    raw_data: Optional[Any] = Field(None, description="Additional unmapped fields")

    class Config:
        """Model configuration."""
        table_name = "stream_mac_apps"
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