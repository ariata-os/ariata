"""
Auto-generated Pydantic model for notion_pages stream.

Generated from: sources/notion/pages/_stream.yaml
Generated at: 2025-08-16T18:38:21.823735
"""

from pydantic import BaseModel, Field
from datetime import datetime, date, time
from typing import Optional, Dict, List, Any
from uuid import UUID


class StreamNotionPages(BaseModel):
    """Pages and databases from Notion workspace"""

    # Required fields
    page_id: str = Field(..., description="Notion page UUID", max_length=100)

    # Optional fields
    parent_id: Optional[str] = Field(None, description="Parent page or workspace ID", max_length=100)
    parent_type: Optional[str] = Field(None, description="Type of parent (page, database, workspace)", max_length=20)
    title: Optional[str] = Field(None, description="Page title", max_length=500)
    object_type: Optional[str] = Field(None, description="Object type (page, database)", max_length=20)
    archived: Optional[bool] = Field(None, description="Whether the page is archived")
    url: Optional[str] = Field(None, description="Public URL if shared", max_length=500)
    created_time: Optional[datetime] = Field(None, description="When the page was created in Notion")
    created_by: Optional[str] = Field(None, description="User ID who created the page", max_length=100)
    last_edited_time: Optional[datetime] = Field(None, description="When the page was last edited in Notion")
    last_edited_by: Optional[str] = Field(None, description="User ID who last edited the page", max_length=100)
    content_text: Optional[str] = Field(None, description="Extracted plain text content")
    content_markdown: Optional[str] = Field(None, description="Content converted to Markdown")
    properties: Optional[Any] = Field(None, description="Database properties and values")
    icon: Optional[Any] = Field(None, description="Page icon (emoji or image)")
    cover: Optional[Any] = Field(None, description="Page cover image")
    parent: Optional[Any] = Field(None, description="Full parent relationship data")
    blocks: Optional[Any] = Field(None, description="Page content blocks (if small)")
    minio_path: Optional[str] = Field(None, description="Path to full content in MinIO (if large)", max_length=500)
    full_page: Optional[Any] = Field(None, description="Complete page object for unmapped fields")

    class Config:
        """Model configuration."""
        table_name = "stream_notion_pages"
        storage_strategy = "postgres_only"
        minio_fields = ['blocks', 'attachments']
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