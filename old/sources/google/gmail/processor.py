"""Gmail data processor for transforming and storing messages."""

import json
import logging
from datetime import datetime, timezone
from typing import Dict, Any, List, Optional
from uuid import uuid4

from sources.base.processors.base import BaseProcessor
from sources.base.storage.minio import MinioClient
from .models import GmailMessage


logger = logging.getLogger(__name__)


class GmailProcessor(BaseProcessor):
    """Process Gmail messages for storage."""
    
    def __init__(self, stream_id: str, source_id: str, user_id: str):
        """Initialize the Gmail processor."""
        super().__init__(stream_id, source_id, user_id)
        self.table_name = "stream_google_gmail"
        
    async def process_data(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process Gmail sync data.
        
        Args:
            data: Raw data from GmailSync containing messages and metadata
            
        Returns:
            Processing result with counts and status
        """
        messages = data.get('messages', [])
        deleted_message_ids = data.get('deleted_message_ids', [])
        history_id = data.get('history_id')
        sync_type = data.get('sync_type', 'unknown')
        
        logger.info(f"Processing {len(messages)} Gmail messages (sync type: {sync_type})")
        
        processed = 0
        failed = 0
        records_to_insert = []
        
        for message in messages:
            try:
                # Convert to dict if it's a Pydantic model
                if isinstance(message, GmailMessage):
                    msg_data = message.dict()
                else:
                    msg_data = message
                
                # Prepare record for database insertion
                record = self._prepare_record(msg_data)
                
                # Store large fields in MinIO if configured
                if self.use_minio and (msg_data.get('body_html') or msg_data.get('full_message')):
                    record = await self._store_large_fields(record, msg_data)
                
                records_to_insert.append(record)
                processed += 1
                
            except Exception as e:
                logger.error(f"Failed to process message {message.get('message_id', 'unknown')}: {e}")
                failed += 1
        
        # Batch insert records
        if records_to_insert:
            try:
                await self.batch_insert(records_to_insert, upsert=True)
                logger.info(f"Inserted/updated {len(records_to_insert)} Gmail messages")
            except Exception as e:
                logger.error(f"Failed to insert records: {e}")
                failed += len(records_to_insert)
                processed = 0
        
        # Handle deleted messages
        if deleted_message_ids:
            try:
                await self._mark_messages_deleted(deleted_message_ids)
                logger.info(f"Marked {len(deleted_message_ids)} messages as deleted")
            except Exception as e:
                logger.error(f"Failed to mark messages as deleted: {e}")
        
        # Update stream state with latest history ID
        if history_id:
            await self.update_stream_state({'history_id': history_id})
            logger.info(f"Updated stream state with history ID: {history_id}")
        
        return {
            'status': 'success' if failed == 0 else 'partial',
            'processed': processed,
            'failed': failed,
            'total': len(messages),
            'deleted': len(deleted_message_ids),
            'sync_type': sync_type,
            'history_id': history_id
        }
    
    def _prepare_record(self, message: Dict[str, Any]) -> Dict[str, Any]:
        """
        Prepare a message record for database insertion.
        
        Args:
            message: Gmail message data
            
        Returns:
            Record dict ready for database
        """
        # Use received_date as the primary timestamp
        timestamp = message.get('received_date')
        if isinstance(timestamp, str):
            timestamp = datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
        elif not timestamp:
            timestamp = datetime.now(timezone.utc)
        
        # Ensure sent_date is also a datetime if present
        sent_date = message.get('sent_date')
        if sent_date and isinstance(sent_date, str):
            sent_date = datetime.fromisoformat(sent_date.replace('Z', '+00:00'))
        
        return {
            'id': str(uuid4()),
            'stream_id': self.stream_id,
            'source_id': self.source_id,
            'user_id': self.user_id,
            'timestamp': timestamp,
            'created_at': datetime.now(timezone.utc),
            'updated_at': datetime.now(timezone.utc),
            
            # Gmail-specific fields
            'message_id': message['message_id'],
            'thread_id': message['thread_id'],
            'history_id': message.get('history_id'),
            
            'subject': message.get('subject'),
            'snippet': message.get('snippet'),
            'body_text': message.get('body_text'),
            'body_html': message.get('body_html'),
            
            'from_email': message.get('from_email'),
            'from_name': message.get('from_name'),
            'to_emails': json.dumps(message.get('to_emails', [])),
            'cc_emails': json.dumps(message.get('cc_emails', [])),
            'bcc_emails': json.dumps(message.get('bcc_emails', [])),
            'reply_to_emails': json.dumps(message.get('reply_to_emails', [])),
            
            'labels': json.dumps(message.get('labels', [])),
            'categories': json.dumps(message.get('categories', [])),
            
            'is_read': message.get('is_read', False),
            'is_starred': message.get('is_starred', False),
            'is_important': message.get('is_important', False),
            'is_spam': message.get('is_spam', False),
            'is_trash': message.get('is_trash', False),
            
            'has_attachments': message.get('has_attachments', False),
            'attachment_count': message.get('attachment_count', 0),
            'attachments': json.dumps(message.get('attachments', [])),
            
            'size_bytes': message.get('size_bytes'),
            'received_date': timestamp,
            'sent_date': sent_date,
            
            'headers': json.dumps(message.get('headers', {})),
            'full_message': json.dumps(message.get('full_message'))
        }
    
    async def _store_large_fields(self, record: Dict[str, Any], message: Dict[str, Any]) -> Dict[str, Any]:
        """
        Store large fields in MinIO and replace with references.
        
        Args:
            record: Database record
            message: Original message data
            
        Returns:
            Updated record with MinIO references
        """
        minio_client = MinioClient()
        
        # Store HTML body in MinIO if it's large
        if message.get('body_html') and len(message['body_html']) > 10000:
            html_key = f"gmail/{self.user_id}/{record['message_id']}/body.html"
            await minio_client.upload_json(
                bucket='ariata',
                key=html_key,
                data={'body_html': message['body_html']}
            )
            record['body_html'] = f"minio://{html_key}"
        
        # Store full message in MinIO if it's large
        if message.get('full_message'):
            full_msg_str = json.dumps(message['full_message'])
            if len(full_msg_str) > 50000:
                msg_key = f"gmail/{self.user_id}/{record['message_id']}/full_message.json"
                await minio_client.upload_json(
                    bucket='ariata',
                    key=msg_key,
                    data=message['full_message']
                )
                record['full_message'] = json.dumps({'minio_ref': f"minio://{msg_key}"})
        
        return record
    
    async def _mark_messages_deleted(self, message_ids: List[str]) -> None:
        """
        Mark messages as deleted in the database.
        
        Args:
            message_ids: List of Gmail message IDs to mark as deleted
        """
        # For now, we'll just log this
        # In production, you might want to either:
        # 1. Delete the records entirely
        # 2. Add a 'deleted' flag to the schema
        # 3. Move to a separate deleted_messages table
        logger.info(f"Would mark as deleted: {message_ids}")
        
        # Example implementation (requires adding 'is_deleted' column to schema):
        # for msg_id in message_ids:
        #     await self.db.execute(
        #         f"UPDATE {self.table_name} SET is_deleted = true WHERE message_id = :msg_id",
        #         {"msg_id": msg_id}
        #     )