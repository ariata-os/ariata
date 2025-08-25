"""Generated from Drizzle schema - DO NOT EDIT MANUALLY"""
from datetime import datetime
from sources.base.models.base import Base
from sqlalchemy import Column, String, Boolean, Integer, Float, DateTime, Text, ForeignKey, JSON
from sqlalchemy.dialects.postgresql import UUID, ARRAY
from uuid import uuid4


class Users(Base):
    """Auto-generated from Drizzle schema."""
    
    __tablename__ = "users"
    
    id = Column(UUID(as_uuid=True), primary_key=True, nullable=False)
    first_name = Column(String, nullable=False)
    last_name = Column(String, nullable=False)
    email = Column(String, nullable=False)
    timezone = Column(String, nullable=False, default='America/Chicago')
    created_at = Column(DateTime, nullable=False, default=datetime.now)
    updated_at = Column(DateTime, nullable=False, default=datetime.now)
