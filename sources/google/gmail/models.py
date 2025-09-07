"""Pydantic models for Gmail data."""

from typing import List, Optional, Dict, Any
from datetime import datetime
from pydantic import BaseModel, Field


class EmailAddress(BaseModel):
    """Email address with optional name."""
    email: str
    name: Optional[str] = None
    
    @classmethod
    def from_header(cls, header: str) -> 'EmailAddress':
        """Parse email from header like 'John Doe <john@example.com>'."""
        import re
        
        # Try to extract name and email
        match = re.match(r'^(.*?)\s*<(.+?)>$', header.strip())
        if match:
            return cls(name=match.group(1).strip('" '), email=match.group(2))
        else:
            # Just an email address
            return cls(email=header.strip())


class AttachmentMetadata(BaseModel):
    """Metadata for an email attachment."""
    filename: str
    mime_type: str
    size: int
    attachment_id: str


class GmailMessage(BaseModel):
    """Gmail message model."""
    message_id: str
    thread_id: str
    history_id: Optional[str] = None
    
    # Email metadata
    subject: Optional[str] = None
    snippet: Optional[str] = None
    
    # Body content
    body_text: Optional[str] = None
    body_html: Optional[str] = None
    
    # Participants
    from_email: Optional[str] = None
    from_name: Optional[str] = None
    to_emails: List[str] = Field(default_factory=list)
    cc_emails: List[str] = Field(default_factory=list)
    bcc_emails: List[str] = Field(default_factory=list)
    reply_to_emails: List[str] = Field(default_factory=list)
    
    # Labels and categories
    labels: List[str] = Field(default_factory=list)
    categories: List[str] = Field(default_factory=list)
    
    # Status flags
    is_read: bool = False
    is_starred: bool = False
    is_important: bool = False
    is_spam: bool = False
    is_trash: bool = False
    
    # Attachments
    has_attachments: bool = False
    attachment_count: int = 0
    attachments: List[AttachmentMetadata] = Field(default_factory=list)
    
    # Size and dates
    size_bytes: Optional[int] = None
    received_date: datetime
    sent_date: Optional[datetime] = None
    
    # Original data
    headers: Dict[str, str] = Field(default_factory=dict)
    full_message: Optional[Dict[str, Any]] = None
    
    @classmethod
    def from_api_response(cls, message: Dict[str, Any], client) -> 'GmailMessage':
        """
        Create GmailMessage from Gmail API response.
        
        Args:
            message: Raw message from Gmail API
            client: GmailClient instance for parsing
            
        Returns:
            GmailMessage instance
        """
        # Parse headers
        headers = client.parse_headers(message)
        
        # Parse body
        text_body, html_body = client.parse_message_body(message)
        
        # Parse participants
        from_header = headers.get('from', '')
        from_addr = EmailAddress.from_header(from_header)
        
        def parse_email_list(header: str) -> List[str]:
            """Parse comma-separated email addresses."""
            if not header:
                return []
            emails = []
            for addr in header.split(','):
                parsed = EmailAddress.from_header(addr.strip())
                if parsed.email:
                    emails.append(parsed.email)
            return emails
            
        # Extract labels
        label_ids = message.get('labelIds', [])
        
        # Determine categories (Gmail's built-in categories)
        categories = []
        category_map = {
            'CATEGORY_PERSONAL': 'Personal',
            'CATEGORY_SOCIAL': 'Social', 
            'CATEGORY_PROMOTIONS': 'Promotions',
            'CATEGORY_UPDATES': 'Updates',
            'CATEGORY_FORUMS': 'Forums'
        }
        for label in label_ids:
            if label in category_map:
                categories.append(category_map[label])
                
        # Parse dates
        internal_date = message.get('internalDate')
        received_date = datetime.fromtimestamp(int(internal_date) / 1000) if internal_date else datetime.utcnow()
        
        # Try to parse sent date from headers
        sent_date = None
        if 'date' in headers:
            try:
                from email.utils import parsedate_to_datetime
                sent_date = parsedate_to_datetime(headers['date'])
            except:
                pass
                
        # Extract attachments
        attachments = client.extract_attachments(message)
        
        return cls(
            message_id=message['id'],
            thread_id=message['threadId'],
            history_id=message.get('historyId'),
            
            subject=headers.get('subject'),
            snippet=message.get('snippet'),
            
            body_text=text_body,
            body_html=html_body,
            
            from_email=from_addr.email,
            from_name=from_addr.name,
            to_emails=parse_email_list(headers.get('to', '')),
            cc_emails=parse_email_list(headers.get('cc', '')),
            bcc_emails=parse_email_list(headers.get('bcc', '')),
            reply_to_emails=parse_email_list(headers.get('reply_to', '')),
            
            labels=label_ids,
            categories=categories,
            
            is_read='UNREAD' not in label_ids,
            is_starred='STARRED' in label_ids,
            is_important='IMPORTANT' in label_ids,
            is_spam='SPAM' in label_ids,
            is_trash='TRASH' in label_ids,
            
            has_attachments=len(attachments) > 0,
            attachment_count=len(attachments),
            attachments=[AttachmentMetadata(**att) for att in attachments],
            
            size_bytes=message.get('sizeEstimate'),
            received_date=received_date,
            sent_date=sent_date,
            
            headers=headers,
            full_message=message
        )


class GmailSyncResult(BaseModel):
    """Result of a Gmail sync operation."""
    messages: List[GmailMessage]
    next_page_token: Optional[str] = None
    history_id: Optional[str] = None
    messages_added: int = 0
    messages_updated: int = 0
    messages_deleted: int = 0
    total_messages: int = 0