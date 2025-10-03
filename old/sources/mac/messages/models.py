"""
Pydantic model for mac_messages stream.

This file will be auto-generated from sources/mac/messages/_stream.yaml
For now, this is a manually created version following the pattern.
"""

from pydantic import BaseModel, Field
from datetime import datetime
from typing import Optional, Dict, List, Any
from uuid import UUID


class StreamMacMessages(BaseModel):
    """iMessage and SMS messages from macOS"""

    # Required fields
    message_id: str = Field(..., description="Unique message identifier (GUID)", max_length=200)
    chat_id: str = Field(..., description="Chat/conversation identifier", max_length=200)
    date: datetime = Field(..., description="Message timestamp")
    is_from_me: bool = Field(False, description="Whether message was sent by user")
    
    # Text content - required but can be empty string for attachments-only messages
    text: Optional[str] = Field(None, description="Message content")

    # Optional fields
    handle_id: Optional[str] = Field(None, description="Contact handle (phone/email)", max_length=200)
    service: Optional[str] = Field("iMessage", description="Service type (iMessage, SMS)", max_length=50)
    date_read: Optional[datetime] = Field(None, description="When message was read")
    date_delivered: Optional[datetime] = Field(None, description="When message was delivered")
    is_read: Optional[bool] = Field(False, description="Whether message has been read")
    is_delivered: Optional[bool] = Field(False, description="Whether message was delivered")
    is_sent: Optional[bool] = Field(False, description="Whether message was sent successfully")
    cache_has_attachments: Optional[bool] = Field(False, description="Whether message has attachments")
    attachment_count: Optional[int] = Field(None, description="Number of attachments")
    attachment_info: Optional[List[Dict[str, Any]]] = Field(None, description="Attachment metadata (filenames, types, sizes)")
    group_title: Optional[str] = Field(None, description="Group chat name if applicable", max_length=500)
    associated_message_guid: Optional[str] = Field(None, description="Related message ID (for replies/reactions)", max_length=200)
    associated_message_type: Optional[int] = Field(None, description="Type of association (reply, reaction, etc)")
    expressive_send_style_id: Optional[str] = Field(None, description="Message effect style (invisible ink, etc)", max_length=100)
    raw_data: Optional[Dict[str, Any]] = Field(None, description="Additional unmapped fields")

    class Config:
        """Model configuration."""
        table_name = "stream_mac_messages"
        storage_strategy = "hybrid"
        minio_fields = ["attachment_info", "raw_data"]
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