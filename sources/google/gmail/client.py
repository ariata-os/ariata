"""Gmail API client."""

import base64
import json
from typing import Dict, Any, List, Optional, Tuple
from datetime import datetime, timezone
import httpx
import logging

logger = logging.getLogger(__name__)


class GmailClient:
    """Client for interacting with Gmail API."""
    
    def __init__(self, access_token: str, token_refresher=None):
        """
        Initialize Gmail client.
        
        Args:
            access_token: OAuth2 access token
            token_refresher: Optional async function to refresh token
        """
        self.access_token = access_token
        self.token_refresher = token_refresher
        self.base_url = "https://gmail.googleapis.com/gmail/v1"
        self.headers = {
            "Authorization": f"Bearer {access_token}",
            "Accept": "application/json"
        }
        
    async def _make_request(self, method: str, endpoint: str, **kwargs) -> Dict[str, Any]:
        """Make authenticated request to Gmail API."""
        url = f"{self.base_url}/users/me{endpoint}"
        
        async with httpx.AsyncClient() as client:
            try:
                response = await client.request(
                    method,
                    url,
                    headers=self.headers,
                    timeout=30.0,
                    **kwargs
                )
                response.raise_for_status()
                return response.json()
                
            except httpx.HTTPStatusError as e:
                if e.response.status_code == 401 and self.token_refresher:
                    # Token expired, refresh and retry
                    logger.info("Access token expired, refreshing...")
                    new_token = await self.token_refresher()
                    self.access_token = new_token
                    self.headers["Authorization"] = f"Bearer {new_token}"
                    
                    # Retry request with new token
                    response = await client.request(
                        method,
                        url,
                        headers=self.headers,
                        timeout=30.0,
                        **kwargs
                    )
                    response.raise_for_status()
                    return response.json()
                else:
                    raise
                    
    async def list_messages(
        self, 
        query: str = "", 
        max_results: int = 500,
        page_token: Optional[str] = None,
        include_spam_trash: bool = False
    ) -> Dict[str, Any]:
        """
        List message IDs matching query.
        
        Args:
            query: Gmail search query (e.g., "after:2023/1/1")
            max_results: Maximum number of results per page
            page_token: Token for pagination
            include_spam_trash: Whether to include spam and trash
            
        Returns:
            Dict with 'messages' list and optional 'nextPageToken'
        """
        params = {
            "maxResults": min(max_results, 500),  # API limit is 500
            "includeSpamTrash": str(include_spam_trash).lower()
        }
        
        if query:
            params["q"] = query
        if page_token:
            params["pageToken"] = page_token
            
        return await self._make_request("GET", "/messages", params=params)
        
    async def get_message(
        self, 
        message_id: str,
        format: str = "full",
        metadata_headers: Optional[List[str]] = None
    ) -> Dict[str, Any]:
        """
        Get a single message.
        
        Args:
            message_id: Gmail message ID
            format: Message format (minimal, full, raw, metadata)
            metadata_headers: Headers to include if format is metadata
            
        Returns:
            Message object
        """
        params = {"format": format}
        
        if metadata_headers and format == "metadata":
            params["metadataHeaders"] = ",".join(metadata_headers)
            
        return await self._make_request("GET", f"/messages/{message_id}", params=params)
        
    async def batch_get_messages(
        self,
        message_ids: List[str],
        format: str = "full"
    ) -> List[Dict[str, Any]]:
        """
        Batch get multiple messages.
        
        Note: Gmail doesn't have a true batch API endpoint, so we'll
        make parallel requests for efficiency.
        
        Args:
            message_ids: List of message IDs to fetch
            format: Message format
            
        Returns:
            List of message objects
        """
        import asyncio
        
        # Create tasks for parallel fetching
        tasks = [
            self.get_message(msg_id, format=format)
            for msg_id in message_ids
        ]
        
        # Fetch messages in parallel (limit concurrency to avoid rate limits)
        messages = []
        for i in range(0, len(tasks), 10):  # Process 10 at a time
            batch = tasks[i:i+10]
            batch_results = await asyncio.gather(*batch, return_exceptions=True)
            
            for result in batch_results:
                if isinstance(result, Exception):
                    logger.error(f"Failed to fetch message: {result}")
                else:
                    messages.append(result)
                    
        return messages
        
    async def get_history(
        self,
        start_history_id: str,
        max_results: int = 500,
        page_token: Optional[str] = None,
        history_types: Optional[List[str]] = None
    ) -> Dict[str, Any]:
        """
        Get history of changes since a given history ID.
        
        Args:
            start_history_id: History ID to start from
            max_results: Maximum results per page
            page_token: Token for pagination
            history_types: Types of history to return (messageAdded, messageDeleted, labelAdded, labelRemoved)
            
        Returns:
            History response with changes
        """
        params = {
            "startHistoryId": start_history_id,
            "maxResults": min(max_results, 500)
        }
        
        if page_token:
            params["pageToken"] = page_token
            
        if history_types:
            params["historyTypes"] = ",".join(history_types)
        else:
            # Default to all message changes
            params["historyTypes"] = "messageAdded,messageDeleted,labelAdded,labelRemoved"
            
        return await self._make_request("GET", "/history", params=params)
        
    async def get_profile(self) -> Dict[str, Any]:
        """
        Get user's Gmail profile (includes current history ID).
        
        Returns:
            Profile with emailAddress, messagesTotal, threadsTotal, historyId
        """
        return await self._make_request("GET", "/profile")
        
    def parse_message_body(self, message: Dict[str, Any]) -> Tuple[Optional[str], Optional[str]]:
        """
        Extract plain text and HTML body from a message.
        
        Args:
            message: Gmail message object
            
        Returns:
            Tuple of (plain_text, html_body)
        """
        def decode_base64(data: str) -> str:
            """Decode base64url encoded data."""
            # Replace URL-safe characters
            data = data.replace('-', '+').replace('_', '/')
            # Add padding if needed
            padding = 4 - len(data) % 4
            if padding != 4:
                data += '=' * padding
            return base64.b64decode(data).decode('utf-8', errors='ignore')
            
        def extract_from_parts(parts: List[Dict]) -> Tuple[Optional[str], Optional[str]]:
            """Recursively extract body from message parts."""
            text_body = None
            html_body = None
            
            for part in parts:
                mime_type = part.get('mimeType', '')
                
                if mime_type == 'text/plain' and not text_body:
                    if 'data' in part.get('body', {}):
                        text_body = decode_base64(part['body']['data'])
                        
                elif mime_type == 'text/html' and not html_body:
                    if 'data' in part.get('body', {}):
                        html_body = decode_base64(part['body']['data'])
                        
                elif mime_type.startswith('multipart/'):
                    # Recursively process nested parts
                    if 'parts' in part:
                        nested_text, nested_html = extract_from_parts(part['parts'])
                        if nested_text and not text_body:
                            text_body = nested_text
                        if nested_html and not html_body:
                            html_body = nested_html
                            
            return text_body, html_body
            
        payload = message.get('payload', {})
        
        # Simple message with body directly in payload
        if 'body' in payload and 'data' in payload['body']:
            content = decode_base64(payload['body']['data'])
            mime_type = payload.get('mimeType', '')
            
            if mime_type == 'text/html':
                return None, content
            else:
                return content, None
                
        # Multipart message
        if 'parts' in payload:
            return extract_from_parts(payload['parts'])
            
        return None, None
        
    def parse_headers(self, message: Dict[str, Any]) -> Dict[str, str]:
        """
        Extract common headers from message.
        
        Args:
            message: Gmail message object
            
        Returns:
            Dict of header values
        """
        headers = {}
        header_list = message.get('payload', {}).get('headers', [])
        
        # Headers we're interested in
        interesting_headers = {
            'From', 'To', 'Cc', 'Bcc', 'Reply-To',
            'Subject', 'Date', 'Message-ID',
            'In-Reply-To', 'References'
        }
        
        for header in header_list:
            name = header.get('name', '')
            if name in interesting_headers:
                headers[name.lower().replace('-', '_')] = header.get('value', '')
                
        return headers
        
    def extract_attachments(self, message: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        Extract attachment metadata from message.
        
        Args:
            message: Gmail message object
            
        Returns:
            List of attachment metadata dicts
        """
        attachments = []
        
        def process_parts(parts: List[Dict]):
            """Recursively process message parts for attachments."""
            for part in parts:
                filename = part.get('filename', '')
                
                if filename:
                    # This is an attachment
                    attachments.append({
                        'filename': filename,
                        'mime_type': part.get('mimeType', ''),
                        'size': part.get('body', {}).get('size', 0),
                        'attachment_id': part.get('body', {}).get('attachmentId', '')
                    })
                    
                # Recursively check nested parts
                if 'parts' in part:
                    process_parts(part['parts'])
                    
        payload = message.get('payload', {})
        if 'parts' in payload:
            process_parts(payload['parts'])
            
        return attachments