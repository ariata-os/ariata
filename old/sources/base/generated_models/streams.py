"""Generated from Drizzle schema - DO NOT EDIT MANUALLY"""
from datetime import datetime
from sources.base.models.base import Base
from sqlalchemy import Column, String, Boolean, Integer, Float, DateTime, Text, ForeignKey, JSON
from sqlalchemy.dialects.postgresql import UUID, ARRAY
from typing import Dict, Any, Optional
from uuid import uuid4


class Streams(Base):
    """Auto-generated from Drizzle schema."""
    
    __tablename__ = "streams"
    
    id = Column(UUID(as_uuid=True), primary_key=True, nullable=False)
    source_id = Column(UUID(as_uuid=True), ForeignKey('sources.id', ondelete='CASCADE'), nullable=False, index=True)
    stream_config_id = Column(UUID(as_uuid=True), ForeignKey('streamConfigs.id', ondelete='RESTRICT'), nullable=False, index=True)
    enabled = Column(Boolean, nullable=False, default=True)
    sync_schedule = Column(String)
    initial_sync_type = Column(String, default='limited')
    initial_sync_days = Column(Integer, default=90)
    initial_sync_days_future = Column(Integer, default=30)
    settings = Column(JSON, default='{}')
    last_sync_at = Column(DateTime)
    last_sync_status = Column(String)
    last_sync_error = Column(String)
    sync_cursor = Column(String)
    created_at = Column(DateTime, nullable=False, default=datetime.now)
    updated_at = Column(DateTime, nullable=False, default=datetime.now)
