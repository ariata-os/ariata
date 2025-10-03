"""Generated from Drizzle schema - DO NOT EDIT MANUALLY"""
from datetime import datetime
from sources.base.models.base import Base
from sqlalchemy import Column, String, Boolean, Integer, Float, DateTime, Text, ForeignKey, JSON
from sqlalchemy.dialects.postgresql import UUID, ARRAY
from typing import Dict, Any, Optional


class SourceConfigs(Base):
    """Auto-generated from Drizzle schema."""
    
    __tablename__ = "source_configs"
    
    name = Column(String, primary_key=True, nullable=False)
    company = Column(String, nullable=False)
    platform = Column(String, nullable=False, default='cloud')
    device_type = Column(String)
    default_fidelity_score = Column(Float, nullable=False, default=1.0)
    auth_type = Column(String, default='oauth2')
    display_name = Column(String)
    description = Column(String)
    icon = Column(String)
    video = Column(String)
    oauth_config = Column(JSON)
    sync_config = Column(JSON)
    default_sync_schedule = Column(String)
    min_sync_frequency = Column(Integer)
    max_sync_frequency = Column(Integer)
    created_at = Column(DateTime, nullable=False, default=datetime.now)
    updated_at = Column(DateTime, nullable=False, default=datetime.now)
