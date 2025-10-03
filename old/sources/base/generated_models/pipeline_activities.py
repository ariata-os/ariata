"""Generated from Drizzle schema - DO NOT EDIT MANUALLY"""
from datetime import datetime
from enum import Enum
from sources.base.models.base import Base
from sqlalchemy import Column, String, Boolean, Integer, Float, DateTime, Text, ForeignKey, JSON
from sqlalchemy.dialects.postgresql import ENUM as PGEnum
from sqlalchemy.dialects.postgresql import UUID, ARRAY
from typing import Dict, Any, Optional
from uuid import uuid4


class ActivityTypeStatus(str, Enum):
    """activity_type enum values."""
    INGESTION = "ingestion"
    SIGNAL_CREATION = "signal_creation"
    TRANSITION_DETECTION = "transition_detection"
    TOKEN_REFRESH = "token_refresh"
    SCHEDULED_CHECK = "scheduled_check"
    CLEANUP = "cleanup"

class ActivityStatus(str, Enum):
    """activity_status enum values."""
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"



class PipelineActivities(Base):
    """Auto-generated from Drizzle schema."""
    
    __tablename__ = "pipeline_activities"
    
    id = Column(UUID(as_uuid=True), primary_key=True, nullable=False)
    activity_type = Column(PGEnum(ActivityTypeStatus, name='activity_type'), nullable=False)
    activity_name = Column(String, nullable=False)
    stream_id = Column(UUID(as_uuid=True), ForeignKey('streams.id', ondelete='SET NULL'), index=True)
    source_name = Column(String, nullable=False)
    status = Column(PGEnum(ActivityStatus, name='activity_status'), nullable=False, default=ActivityStatus.PENDING)
    started_at = Column(DateTime)
    completed_at = Column(DateTime)
    records_processed = Column(Integer)
    data_size_bytes = Column(Integer)
    output_path = Column(String)
    activity_metadata = Column(JSON)
    error_message = Column(String)
    created_at = Column(DateTime, nullable=False, default=datetime.now)
    updated_at = Column(DateTime, nullable=False, default=datetime.now)
