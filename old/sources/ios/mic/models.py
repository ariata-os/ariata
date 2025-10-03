"""
Auto-generated Pydantic model for ios_mic stream.

Generated from: sources/ios/mic/_stream.yaml
Generated at: 2025-08-16T18:38:21.804545
"""

from pydantic import BaseModel, Field
from datetime import datetime, date, time
from typing import Optional, Dict, List, Any
from uuid import UUID


class StreamIosMic(BaseModel):
    """Audio metadata and transcriptions from iOS microphone"""

    # Required fields
    recording_id: str = Field(..., description="Unique identifier for the recording", max_length=100)
    timestamp_start: datetime = Field(..., description="Start time of the recording")
    timestamp_end: datetime = Field(..., description="End time of the recording")
    duration: int = Field(..., description="Duration in milliseconds")

    # Optional fields
    overlap_duration: Optional[float] = Field(None, description="Overlap duration with previous recording in seconds")
    audio_format: Optional[str] = Field(None, description="Audio format (wav, mp3, etc.)", max_length=10)
    sample_rate: Optional[int] = Field(None, description="Sample rate in Hz")
    audio_level: Optional[float] = Field(None, description="Average audio level in dB")
    peak_level: Optional[float] = Field(None, description="Peak audio level in dB")
    transcription_text: Optional[str] = Field(None, description="Transcribed text from audio")
    transcription_confidence: Optional[float] = Field(None, description="Confidence score of transcription")
    language: Optional[str] = Field(None, description="Detected language code", max_length=10)
    minio_path: Optional[str] = Field(None, description="Path to audio file in MinIO storage", max_length=500)
    file_size: Optional[int] = Field(None, description="Size of audio file in bytes")
    raw_data: Optional[Any] = Field(None, description="Additional metadata")

    class Config:
        """Model configuration."""
        table_name = "stream_ios_mic"
        storage_strategy = "postgres_only"
        minio_fields = ['audio_data']
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