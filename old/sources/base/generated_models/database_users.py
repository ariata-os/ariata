"""Generated from Drizzle schema - DO NOT EDIT MANUALLY"""
from datetime import datetime
from sources.base.models.base import Base
from sqlalchemy import Column, String, Boolean, Integer, Float, DateTime, Text, ForeignKey, JSON
from sqlalchemy.dialects.postgresql import UUID, ARRAY
from typing import Dict, Any, Optional
from uuid import uuid4


class DatabaseUsers(Base):
    """Auto-generated from Drizzle schema."""
    
    __tablename__ = "database_users"
    
    id = Column(UUID(as_uuid=True), primary_key=True, nullable=False)
    name = Column(String, nullable=False)
    permission_level = Column(String, nullable=False)
    connection_string_encrypted = Column(String, nullable=False)
    permissions = Column(JSON, nullable=False)
    created_at = Column(DateTime, nullable=False, default=datetime.now)
    last_used = Column(DateTime)
    revoked_at = Column(DateTime)
