"""Gmail incremental sync logic using History API."""

import json
import asyncio
from datetime import datetime, timedelta, timezone
from typing import Optional, List, Dict, Any, Tuple
from uuid import uuid4
import logging

from .client import GmailClient
from .models import GmailMessage, GmailSyncResult
from sources.base.interfaces.sync import BaseSync


logger = logging.getLogger(__name__)


class GmailSync(BaseSync):
    """Handles incremental sync of Gmail data using History API."""

    requires_credentials = True  # This source requires OAuth credentials

    def __init__(self, stream, access_token: str, token_refresher=None):
        """Initialize the sync handler."""
        super().__init__(stream, access_token, token_refresher)
        self.client = GmailClient(access_token, token_refresher)
        self.stream = stream
        
        # Get sync configuration from stream config
        config = stream.stream_config.get('config', {})
        self.initial_sync_days = config.get('initial_sync_days', 730)  # Default 2 years
        self.batch_size = config.get('batch_size', 50)  # Messages per batch
        self.include_spam_trash = config.get('include_spam_trash', False)
    
    def get_full_sync_range(self) -> Tuple[datetime, datetime]:
        """
        Gmail uses history API, not date ranges.
        Return None to signal history-based sync.
        """
        return (None, None)
    
    def get_incremental_sync_range(self) -> Tuple[Optional[datetime], Optional[datetime]]:
        """
        Gmail uses history API for incremental sync.
        Returns None, None to use history API exclusively.
        """
        return (None, None)
    
    async def fetch_data(self, start_date: Optional[datetime], end_date: Optional[datetime]) -> Dict[str, Any]:
        """
        Fetch data for the given date range.
        If dates are None, uses history API for incremental sync.
        """
        # Check if we have a history ID for incremental sync
        history_id = self.stream.state.get('history_id') if self.stream.state else None
        
        if not history_id:
            # Initial sync
            logger.info("No history ID found, performing initial sync")
            return await self._initial_sync(start_date or self._get_initial_sync_date())
        else:
            # Incremental sync using History API
            logger.info(f"Performing incremental sync from history ID: {history_id}")
            return await self._incremental_sync(history_id)
    
    def _get_initial_sync_date(self) -> datetime:
        """Get the date for initial sync based on configuration."""
        return datetime.now(timezone.utc) - timedelta(days=self.initial_sync_days)
    
    async def _initial_sync(self, start_date: datetime) -> Dict[str, Any]:
        """
        Perform initial sync of Gmail messages.
        
        Args:
            start_date: Start date for fetching messages
            
        Returns:
            Sync result dictionary
        """
        logger.info(f"Starting initial Gmail sync from {start_date}")
        
        # Build query for initial sync
        query = f"after:{start_date.strftime('%Y/%m/%d')}"
        
        # Add label filters if configured
        if hasattr(self, 'initial_labels') and self.initial_labels:
            label_query = ' OR '.join([f'label:{label}' for label in self.initial_labels])
            query = f"({query}) AND ({label_query})"
        
        messages = []
        next_page_token = None
        total_fetched = 0
        latest_history_id = None
        
        # First, get the current history ID from profile
        try:
            profile = await self.client.get_profile()
            latest_history_id = profile.get('historyId')
            logger.info(f"Current history ID: {latest_history_id}")
        except Exception as e:
            logger.error(f"Failed to get profile: {e}")
        
        # Paginate through messages
        while True:
            try:
                # List messages matching query
                response = await self.client.list_messages(
                    query=query,
                    max_results=500,
                    page_token=next_page_token,
                    include_spam_trash=self.include_spam_trash
                )
                
                message_refs = response.get('messages', [])
                if not message_refs:
                    break
                
                # Fetch full message details in batches
                for i in range(0, len(message_refs), self.batch_size):
                    batch_ids = [msg['id'] for msg in message_refs[i:i+self.batch_size]]
                    
                    logger.info(f"Fetching batch of {len(batch_ids)} messages")
                    batch_messages = await self.client.batch_get_messages(batch_ids)
                    
                    # Convert to our model
                    for msg_data in batch_messages:
                        try:
                            message = GmailMessage.from_api_response(msg_data, self.client)
                            messages.append(message)
                            
                            # Track the latest history ID
                            if message.history_id:
                                if not latest_history_id or int(message.history_id) > int(latest_history_id):
                                    latest_history_id = message.history_id
                        except Exception as e:
                            logger.error(f"Failed to parse message {msg_data.get('id')}: {e}")
                    
                    total_fetched += len(batch_ids)
                    logger.info(f"Total messages fetched: {total_fetched}")
                
                # Check for next page
                next_page_token = response.get('nextPageToken')
                if not next_page_token:
                    break
                    
            except Exception as e:
                logger.error(f"Error during initial sync: {e}")
                break
        
        logger.info(f"Initial sync completed. Fetched {len(messages)} messages")
        
        return {
            'messages': messages,
            'history_id': latest_history_id,
            'total_messages': len(messages),
            'sync_type': 'initial'
        }
    
    async def _incremental_sync(self, start_history_id: str) -> Dict[str, Any]:
        """
        Perform incremental sync using Gmail History API.
        
        Args:
            start_history_id: History ID to start from
            
        Returns:
            Sync result dictionary
        """
        logger.info(f"Starting incremental sync from history ID: {start_history_id}")
        
        messages_to_add = []
        messages_to_update = []
        messages_to_delete = []
        latest_history_id = start_history_id
        next_page_token = None
        
        # Process history pages
        while True:
            try:
                # Get history since last sync
                history_response = await self.client.get_history(
                    start_history_id=start_history_id,
                    max_results=500,
                    page_token=next_page_token
                )
                
                # Update latest history ID
                latest_history_id = history_response.get('historyId', latest_history_id)
                
                # Process history records
                history_records = history_response.get('history', [])
                
                for record in history_records:
                    # Handle added messages
                    if 'messagesAdded' in record:
                        for item in record['messagesAdded']:
                            message_id = item['message']['id']
                            if message_id not in [m['id'] for m in messages_to_add]:
                                messages_to_add.append({'id': message_id})
                    
                    # Handle deleted messages
                    if 'messagesDeleted' in record:
                        for item in record['messagesDeleted']:
                            message_id = item['message']['id']
                            if message_id not in messages_to_delete:
                                messages_to_delete.append(message_id)
                    
                    # Handle label changes (treated as updates)
                    if 'labelsAdded' in record or 'labelsRemoved' in record:
                        items = record.get('labelsAdded', []) + record.get('labelsRemoved', [])
                        for item in items:
                            message_id = item['message']['id']
                            if message_id not in [m['id'] for m in messages_to_update]:
                                messages_to_update.append({'id': message_id})
                
                # Check for next page
                next_page_token = history_response.get('nextPageToken')
                if not next_page_token:
                    break
                    
            except Exception as e:
                if 'startHistoryId' in str(e) and 'too old' in str(e).lower():
                    logger.warning(f"History ID too old, need to perform full sync: {e}")
                    # History ID is too old, need to do a fresh sync
                    return await self._initial_sync(self._get_initial_sync_date())
                else:
                    logger.error(f"Error fetching history: {e}")
                    raise
        
        # Fetch full details for new and updated messages
        messages = []
        all_message_ids = [m['id'] for m in messages_to_add + messages_to_update]
        
        if all_message_ids:
            logger.info(f"Fetching details for {len(all_message_ids)} new/updated messages")
            
            # Remove duplicates
            all_message_ids = list(set(all_message_ids))
            
            # Fetch in batches
            for i in range(0, len(all_message_ids), self.batch_size):
                batch_ids = all_message_ids[i:i+self.batch_size]
                batch_messages = await self.client.batch_get_messages(batch_ids)
                
                for msg_data in batch_messages:
                    try:
                        message = GmailMessage.from_api_response(msg_data, self.client)
                        messages.append(message)
                    except Exception as e:
                        logger.error(f"Failed to parse message {msg_data.get('id')}: {e}")
        
        logger.info(f"Incremental sync completed. Added: {len(messages_to_add)}, "
                   f"Updated: {len(messages_to_update)}, Deleted: {len(messages_to_delete)}")
        
        return {
            'messages': messages,
            'deleted_message_ids': messages_to_delete,
            'history_id': latest_history_id,
            'messages_added': len(messages_to_add),
            'messages_updated': len(messages_to_update),
            'messages_deleted': len(messages_to_delete),
            'total_messages': len(messages),
            'sync_type': 'incremental'
        }