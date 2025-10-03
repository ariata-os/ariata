"""Google Calendar API client wrapper."""

from datetime import datetime
from typing import List, Dict, Any, Optional
from urllib.parse import urlencode, quote
from sources.base.clients.base import BaseAPIClient, RateLimiter


class GoogleCalendarClient(BaseAPIClient):
    """Client for interacting with Google Calendar API v3."""
    
    BASE_URL = "https://www.googleapis.com/calendar/v3"
    
    def __init__(self, access_token: str, token_refresher=None):
        """Initialize with OAuth access token."""
        super().__init__(access_token=access_token, token_refresher=token_refresher)
    
    def _create_rate_limiter(self) -> RateLimiter:
        """
        Create Google Calendar API rate limiter.
        
        Google Calendar API has a quota of 1,000,000 queries per day
        and 500 queries per 100 seconds per user.
        We'll limit to 5 requests per second to be conservative.
        
        Returns:
            Configured RateLimiter instance
        """
        return RateLimiter(requests_per_second=5.0)
    
    async def list_calendars(self) -> List[Dict[str, Any]]:
        """List all calendars accessible by the user."""
        response = await self.get("/users/me/calendarList")
        return response.get("items", [])
    
    async def list_events(
        self,
        calendar_id: str = "primary",
        time_min: Optional[datetime] = None,
        time_max: Optional[datetime] = None,
        page_token: Optional[str] = None,
        sync_token: Optional[str] = None,
        max_results: int = 250,
        single_events: bool = True,
        order_by: str = "startTime",
        show_deleted: bool = False
    ) -> Dict[str, Any]:
        """
        List events from a calendar.
        
        Args:
            calendar_id: Calendar identifier (default: "primary")
            time_min: Lower bound for event's end time (ignored if sync_token provided)
            time_max: Upper bound for event's start time (ignored if sync_token provided)
            page_token: Token for pagination
            sync_token: Sync token for incremental sync (overrides time_min/time_max)
            max_results: Maximum events per page (max 2500)
            single_events: Expand recurring events into instances
            order_by: Order results by startTime or updated
            show_deleted: Whether to include deleted events (useful for sync)
        
        Returns:
            Dict containing events, nextPageToken, and nextSyncToken if available
        
        Raises:
            HTTPStatusError: If sync token is invalid (410 Gone), caller should retry with full sync
        """
        params = {
            "maxResults": min(max_results, 2500),
        }
        
        # Sync token takes precedence over all other parameters
        if sync_token:
            params["syncToken"] = sync_token
            # When using sync token, these params are not allowed
            params["showDeleted"] = True  # Always show deleted with sync token
        else:
            # Regular listing parameters
            params["singleEvents"] = single_events
            params["orderBy"] = order_by
            params["showDeleted"] = show_deleted
            
            if time_min:
                # Remove microseconds and timezone info for Google Calendar API compatibility
                if hasattr(time_min, 'replace'):
                    time_min = time_min.replace(microsecond=0)
                    # If timezone-aware, convert to naive UTC
                    if time_min.tzinfo is not None:
                        time_min = time_min.replace(tzinfo=None)
                params["timeMin"] = time_min.isoformat() + "Z"
            if time_max:
                # Remove microseconds and timezone info for Google Calendar API compatibility
                if hasattr(time_max, 'replace'):
                    time_max = time_max.replace(microsecond=0)
                    # If timezone-aware, convert to naive UTC
                    if time_max.tzinfo is not None:
                        time_max = time_max.replace(tzinfo=None)
                params["timeMax"] = time_max.isoformat() + "Z"
                
        if page_token:
            params["pageToken"] = page_token
        
        # Properly URL-encode calendar ID to handle special characters like #
        calendar_id_encoded = quote(calendar_id, safe='')
        
        response = await self.get(
            f"/calendars/{calendar_id_encoded}/events",
            params=params
        )
        return response
    
    async def get_event(self, calendar_id: str, event_id: str) -> Dict[str, Any]:
        """Get a single event by ID."""
        calendar_id_encoded = quote(calendar_id, safe='')
        event_id_encoded = quote(event_id, safe='')
        
        return await self.get(
            f"/calendars/{calendar_id_encoded}/events/{event_id_encoded}"
        )