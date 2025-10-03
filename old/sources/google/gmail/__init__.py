"""Gmail stream for Google source."""

from .sync import GmailSync
from .processor import GmailProcessor

__all__ = ['GmailSync', 'GmailProcessor']