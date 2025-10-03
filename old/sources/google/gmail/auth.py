"""Gmail authentication utilities - reuses Google Calendar auth."""

# Gmail uses the same OAuth tokens as other Google services
# Token refresh is handled at the source level (Google)
from sources.google.calendar.auth import refresh_google_token

__all__ = ['refresh_google_token']