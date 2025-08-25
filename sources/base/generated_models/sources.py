"""Generated from Drizzle schema - DO NOT EDIT MANUALLY"""
from datetime import datetime
from sources.base.models.base import Base
from sqlalchemy import Column, String, Boolean, Integer, Float, DateTime, Text, ForeignKey, JSON
from sqlalchemy.dialects.postgresql import UUID, ARRAY
from typing import Dict, Any, Optional
from uuid import uuid4


class Sources(Base):
    """Auto-generated from Drizzle schema."""
    
    __tablename__ = "sources"
    
    id = Column(UUID(as_uuid=True), primary_key=True, nullable=False)
    source_name = Column(String, ForeignKey('sourceConfigs.name', ondelete='RESTRICT'), nullable=False, index=True)
    instance_name = Column(String, nullable=False)
    status = Column(String, nullable=False, default='authenticated')
    device_id = Column(String)
    device_token = Column(String)
    device_type = Column(String)
    device_last_seen = Column(DateTime)
    oauth_access_token = Column(String)
    oauth_refresh_token = Column(String)
    oauth_expires_at = Column(DateTime)
    scopes = Column(JSON)
    source_metadata = Column(JSON, default='{}')
    last_sync_at = Column(DateTime)
    last_sync_status = Column(String)
    last_sync_error = Column(String)
    created_at = Column(DateTime, nullable=False, default=datetime.now)
    updated_at = Column(DateTime, nullable=False, default=datetime.now)
