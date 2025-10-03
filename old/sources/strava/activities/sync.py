"""Strava Activities incremental sync logic."""

import json
import asyncio
from datetime import datetime, timedelta, timezone
from typing import Optional, List, Dict, Any, Tuple
from uuid import uuid4
import httpx

from .client import StravaClient
from sources.base.storage.minio import store_raw_data
from sources.base.interfaces.sync import BaseSync


class StravaActivitiesSync(BaseSync):
    """Handles incremental sync of Strava activities data."""

    requires_credentials = True  # This source requires OAuth credentials

    def __init__(self, stream, access_token: str, token_refresher=None):
        """Initialize the sync handler."""
        super().__init__(stream, access_token, token_refresher)
        self.client = StravaClient(access_token, token_refresher)
        self.stream = stream
    
    def get_full_sync_range(self) -> Tuple[Optional[datetime], Optional[datetime]]:
        """
        Get date range for full sync.
        For Strava, 'full' means ALL available activities without any date filtering.
        Returns (None, None) to fetch all activities.
        """
        # Return None, None to fetch ALL activities without date filters
        return (None, None)
    
    def get_incremental_sync_range(self) -> Tuple[Optional[datetime], Optional[datetime]]:
        """
        Get date range for incremental sync.
        Uses last sync timestamp if available.
        """
        # Get last sync time from stream metadata
        if hasattr(self.stream, 'last_sync_at') and self.stream.last_sync_at:
            # Sync from last sync time minus 1 hour (for safety)
            start_date = self.stream.last_sync_at - timedelta(hours=1)
            end_date = datetime.now(timezone.utc)
            return (start_date, end_date)
        else:
            # No previous sync, do a full sync
            return self.get_full_sync_range()
    
    async def fetch_data(self, start_date: Optional[datetime], end_date: Optional[datetime]) -> Dict[str, Any]:
        """
        Fetch data for the given date range.
        """
        return await self._run_sync(start_date, end_date)

    async def _run_sync(self, start_date: Optional[datetime], end_date: Optional[datetime]) -> Dict[str, Any]:
        """
        Internal sync implementation.

        Returns:
            Dict with sync statistics (activities_processed, errors, etc.)
        """
        stats = {
            "activities_processed": 0,
            "activities_with_streams": 0,
            "errors": [],
            "started_at": datetime.now(timezone.utc),
            "completed_at": None
        }

        try:
            # Get athlete info first
            athlete = await self.client.get_athlete()
            athlete_id = athlete["id"]
            print(f"Syncing activities for athlete: {athlete.get('firstname')} {athlete.get('lastname')} (ID: {athlete_id})")

            # Convert dates to Unix timestamps for Strava API
            after_timestamp = int(start_date.timestamp()) if start_date else None
            before_timestamp = int(end_date.timestamp()) if end_date else None

            print(f"Fetching activities between {start_date} and {end_date}")

            # Fetch activities with pagination
            all_activities = []
            page = 1
            per_page = 200  # Use maximum allowed by Strava for efficiency

            while True:
                try:
                    activities = await self.client.list_activities(
                        after=after_timestamp,
                        before=before_timestamp,
                        page=page,
                        per_page=per_page
                    )
                    
                    if not activities:
                        break
                    
                    all_activities.extend(activities)
                    print(f"Fetched page {page} with {len(activities)} activities")
                    
                    if len(activities) < per_page:
                        break
                    
                    page += 1
                    
                    # Rate limiting protection
                    await asyncio.sleep(0.5)
                    
                except Exception as e:
                    error_msg = f"Error fetching activities page {page}: {str(e)}"
                    print(error_msg)
                    stats["errors"].append(error_msg)
                    break

            print(f"Total activities fetched: {len(all_activities)}")

            # Process each activity
            for activity_summary in all_activities:
                activity_id = activity_summary["id"]
                
                try:
                    # Get detailed activity data
                    activity_detail = await self.client.get_activity(activity_id)
                    
                    # Try to get activity streams (time series data) if available
                    streams = {}
                    try:
                        streams = await self.client.get_activity_streams(activity_id)
                        if streams:
                            stats["activities_with_streams"] += 1
                    except Exception as e:
                        # Streams might not be available for all activities
                        print(f"Could not fetch streams for activity {activity_id}: {str(e)}")
                    
                    # Try to get activity zones if available
                    zones = []
                    try:
                        zones = await self.client.get_activity_zones(activity_id)
                    except Exception as e:
                        # Zones might not be available
                        print(f"Could not fetch zones for activity {activity_id}: {str(e)}")
                    
                    # Try to get laps if available
                    laps = []
                    try:
                        laps = await self.client.get_activity_laps(activity_id)
                    except Exception as e:
                        # Laps might not be available
                        print(f"Could not fetch laps for activity {activity_id}: {str(e)}")
                    
                    # Combine all data
                    activity_data = {
                        **activity_detail,
                        "streams": streams,
                        "zones": zones,
                        "laps": laps,
                        "_sync_metadata": {
                            "synced_at": datetime.now(timezone.utc).isoformat(),
                            "athlete_id": athlete_id
                        }
                    }
                    
                    # Store raw data in MinIO
                    object_name = await store_raw_data(
                        stream_name="strava_activities",
                        connection_id=str(self.stream.id),
                        data=activity_data,
                        timestamp=datetime.now(timezone.utc)
                    )
                    
                    print(f"Stored activity {activity_id} ({activity_detail.get('name')}) - Type: {activity_detail.get('type')}")
                    stats["activities_processed"] += 1
                    
                    # Rate limiting protection
                    await asyncio.sleep(0.3)
                    
                except Exception as e:
                    error_msg = f"Error processing activity {activity_id}: {str(e)}"
                    print(error_msg)
                    stats["errors"].append(error_msg)
                    continue

            # Get and store athlete stats
            try:
                athlete_stats = await self.client.get_stats()
                
                # Store stats as a separate batch
                await store_raw_data(
                    stream_name="strava_activities",
                    connection_id=str(self.stream.id),
                    data={
                        "athlete_stats": athlete_stats,
                        "athlete_id": athlete_id,
                        "_sync_metadata": {
                            "synced_at": datetime.now(timezone.utc).isoformat(),
                            "type": "athlete_stats"
                        }
                    },
                    timestamp=datetime.now(timezone.utc)
                )
                print("Stored athlete statistics")
            except Exception as e:
                error_msg = f"Error fetching athlete stats: {str(e)}"
                print(error_msg)
                stats["errors"].append(error_msg)

            stats["completed_at"] = datetime.now(timezone.utc)
            
            # Update stream's last sync timestamp
            if hasattr(self.stream, 'last_sync_at'):
                self.stream.last_sync_at = datetime.now(timezone.utc)
            
            print(f"Sync completed: {stats['activities_processed']} activities processed, {len(stats['errors'])} errors")
            
            return stats

        except Exception as e:
            error_msg = f"Fatal error during sync: {str(e)}"
            print(error_msg)
            stats["errors"].append(error_msg)
            stats["completed_at"] = datetime.now(timezone.utc)
            return stats