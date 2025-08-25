"""Google Calendar incremental sync logic."""

import json
import asyncio
from datetime import datetime, timedelta, timezone
from typing import Optional, List, Dict, Any, Tuple
from uuid import uuid4
import httpx

from .client import GoogleCalendarClient
from sources.base.interfaces.sync import BaseSync


class GoogleCalendarSync(BaseSync):
    """Handles incremental sync of Google Calendar data."""

    requires_credentials = True  # This source requires OAuth credentials

    def __init__(self, stream, access_token: str, token_refresher=None):
        """Initialize the sync handler."""
        super().__init__(stream, access_token, token_refresher)
        self.client = GoogleCalendarClient(access_token, token_refresher)
        self.stream = stream
    
    def get_full_sync_range(self) -> Tuple[datetime, datetime]:
        """
        Google Calendar uses sync tokens, not date ranges.
        Return None to signal token-based sync.
        """
        # We don't use date ranges - we use sync tokens
        return (None, None)
    
    def get_incremental_sync_range(self) -> Tuple[Optional[datetime], Optional[datetime]]:
        """
        Google Calendar always uses sync tokens for proper change tracking.
        Returns None, None to use sync tokens exclusively.
        """
        # Always use sync tokens - no date ranges
        return (None, None)
    
    async def fetch_data(self, start_date: Optional[datetime], end_date: Optional[datetime]) -> Dict[str, Any]:
        """
        Fetch data for the given date range.
        If dates are None, uses sync tokens for incremental sync.
        """
        # This wraps the existing run() logic
        return await self._run_sync(start_date, end_date)

    async def _run_sync(self, start_date: Optional[datetime], end_date: Optional[datetime]) -> Dict[str, Any]:
        """
        Internal sync implementation that handles both token and date-based sync.

        Returns:
            Dict with sync statistics (events_processed, errors, etc.)
        """
        stats = {
            "events_processed": 0,
            "calendars_synced": 0,
            "errors": [],
            "started_at": datetime.now(timezone.utc),
            "completed_at": None
        }

        try:
            # List all calendars
            calendars = await self.client.list_calendars()
            
            # Get selected calendar IDs from stream settings
            selected_calendar_ids = None
            if self.stream.settings and "calendar_ids" in self.stream.settings:
                selected_calendar_ids = self.stream.settings["calendar_ids"]
                print(f"Using selected calendars from settings: {selected_calendar_ids}")

            # Load existing sync tokens (stored as JSON dict per calendar)
            existing_sync_tokens = {}
            if hasattr(self.stream, 'sync_token') and self.stream.sync_token:
                try:
                    # Try to parse as JSON dict
                    existing_sync_tokens = json.loads(self.stream.sync_token)
                    print(f"Loaded sync tokens for {len(existing_sync_tokens)} calendars")
                except (json.JSONDecodeError, TypeError):
                    print("Warning: sync_token is not valid JSON")
                    existing_sync_tokens = {}
            
            # Store the new sync tokens we'll save at the end
            calendar_sync_tokens = {}  # Store new sync tokens per calendar

            for calendar in calendars:
                calendar_id = calendar["id"]
                
                # If we have selected calendars, only sync those
                if selected_calendar_ids is not None:
                    if calendar_id not in selected_calendar_ids:
                        print(f"Skipping calendar {calendar_id} - not in selected list")
                        continue
                else:
                    # Use the calendar's 'selected' property as fallback
                    if not calendar.get("selected", True):
                        print(f"Skipping unselected calendar {calendar_id}")
                        continue
                
                print(f"Processing calendar: {calendar_id}")
                stats["calendars_synced"] += 1

                # Try incremental sync with token for THIS specific calendar
                use_sync_token = existing_sync_tokens.get(calendar_id)
                if use_sync_token and use_sync_token.strip():  # Check for non-empty token
                    print(f"Using existing sync token for calendar {calendar_id}: {use_sync_token[:20]}...")
                else:
                    use_sync_token = None  # Ensure it's None, not empty string
                    print(f"No sync token found for calendar {calendar_id}, will do full sync")
                fallback_to_time_sync = False

                # Paginate through events
                page_token = None
                while True:
                    try:
                        if use_sync_token and not fallback_to_time_sync:
                            # Try sync token approach
                            try:
                                result = await self.client.list_events(
                                    calendar_id=calendar_id,
                                    sync_token=use_sync_token,
                                    page_token=page_token
                                )
                            except httpx.HTTPStatusError as e:
                                if e.response.status_code == 410:
                                    # Sync token invalid - do full sync to get new token
                                    print(
                                        f"Sync token expired for calendar {calendar_id}, doing full sync to get new token")
                                    stats["errors"].append({
                                        "calendar_id": calendar_id,
                                        "error": "Sync token expired (410), doing full sync",
                                        "type": "sync_token_expired"
                                    })
                                    fallback_to_time_sync = True  # This now means "do full sync"
                                    use_sync_token = None
                                    page_token = None  # Reset pagination
                                    continue  # Retry with full sync
                                else:
                                    raise
                        else:
                            # No sync token - do a full sync to get one
                            print(f"No sync token for calendar {calendar_id}, doing full sync to get token")
                            
                            # Full sync without date filters to get sync token
                            # Note: Cannot use singleEvents or orderBy with sync tokens
                            result = await self.client.list_events(
                                calendar_id=calendar_id,
                                page_token=page_token,
                                single_events=False,  # MUST be False to get sync tokens
                                order_by="updated",  # Use 'updated' instead of 'startTime'
                                show_deleted=True  # Include deleted events
                            )

                        events = result.get("items", [])
                        print(
                            f"Calendar {calendar_id}: Found {len(events)} events in this page")

                        # Just count events - they're already collected
                        stats["events_processed"] += len(events)

                        # Log progress for large syncs
                        if stats["events_processed"] > 0 and stats["events_processed"] % 100 == 0:
                            print(
                                f"Progress: Processed {stats['events_processed']} events from calendar {calendar_id}")

                        # Collect all events for this calendar
                        if not hasattr(self, '_all_events'):
                            self._all_events = []
                        
                        for event in events:
                            self._all_events.append({
                                "calendar": {
                                    "id": calendar["id"],
                                    "summary": calendar.get("summary"),
                                    "timeZone": calendar.get("timeZone")
                                },
                                "event": event
                            })
                        
                        # Store the sync token from the last page
                        if "nextSyncToken" in result and not result.get("nextPageToken"):
                            calendar_sync_tokens[calendar_id] = result["nextSyncToken"]
                            print(
                                f"Received new sync token for calendar {calendar_id}: {result['nextSyncToken'][:20]}...")
                        else:
                            # Debug: log what we got instead
                            if result.get("nextPageToken"):
                                print(f"Got nextPageToken for calendar {calendar_id}, will fetch next page")
                            else:
                                print(f"No nextSyncToken in result for calendar {calendar_id}. Keys: {list(result.keys())}")

                        # Check for next page
                        page_token = result.get("nextPageToken")
                        if not page_token:
                            print(f"No more pages for calendar {calendar_id}")
                            break

                    except httpx.HTTPStatusError as e:
                        if e.response.status_code == 404:
                            # Calendar not found or no access - skip it
                            print(
                                f"Calendar {calendar_id} not found or no access (404), skipping")
                            stats["errors"].append({
                                "calendar_id": calendar_id,
                                "error": "Calendar not found or no access (404)",
                                "type": "calendar_not_found"
                            })
                            break  # Skip to next calendar
                        else:
                            stats["errors"].append({
                                "calendar_id": calendar_id,
                                "error": str(e),
                                "type": "http_error"
                            })
                            break
                    except Exception as e:
                        stats["errors"].append({
                            "calendar_id": calendar_id,
                            "error": str(e),
                            "type": "general_error"
                        })
                        break

            # Merge new sync tokens with existing ones (for calendars we didn't sync)
            # This preserves tokens for calendars that weren't selected this time
            all_sync_tokens = existing_sync_tokens.copy()
            all_sync_tokens.update(calendar_sync_tokens)
            
            print(f"Total sync tokens after merge: {len(all_sync_tokens)} calendars")

            # Process all events directly to PostgreSQL
            if hasattr(self, '_all_events') and self._all_events:
                # Prepare data for direct processing
                stream_data = {
                    "events": self._all_events,
                    "batch_metadata": {
                        "total_events": len(self._all_events),
                        "calendars_synced": stats["calendars_synced"],
                        "sync_type": "incremental" if calendar_sync_tokens else "full",
                        "fetched_at": datetime.now(timezone.utc).isoformat()
                    }
                }
                
                # Queue direct stream processing via Celery (no MinIO)
                from sources.base.scheduler.celery_app import app
                
                # Create pipeline activity ID for tracking
                pipeline_activity_id = str(uuid4())
                
                # Queue the direct processing task
                # This will process events directly to PostgreSQL
                task = app.send_task(
                    'process_stream_data',
                    args=[
                        'google_calendar',      # stream_name
                        stream_data,            # data directly (not MinIO key)
                        str(self.stream.source_id),  # source_id
                        str(self.stream.id),    # stream_id 
                        pipeline_activity_id    # pipeline_activity_id
                    ]
                )
                
                stats["processing_task_id"] = task.id
                stats["pipeline_activity_id"] = pipeline_activity_id
            
            stats["completed_at"] = datetime.now(timezone.utc)
            # Return sync tokens as JSON string dict
            stats["next_sync_token"] = json.dumps(all_sync_tokens) if all_sync_tokens else None
            stats["is_initial_sync"] = self.stream.last_successful_ingestion_at is None if hasattr(self.stream, 'last_successful_ingestion_at') else True
            
            print(f"Sync complete. Returning {len(all_sync_tokens)} sync tokens for next run")

        except Exception as e:
            stats["errors"].append({
                "error": str(e),
                "type": "sync_error"
            })
            raise

        return stats
