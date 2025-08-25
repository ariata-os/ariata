"""Strava API client wrapper."""

from datetime import datetime
from typing import List, Dict, Any, Optional
from sources.base.clients.base import BaseAPIClient


class StravaClient(BaseAPIClient):
    """Client for interacting with Strava API v3."""
    
    BASE_URL = "https://www.strava.com/api/v3"
    
    def __init__(self, access_token: str):
        """Initialize with OAuth access token."""
        super().__init__(access_token=access_token)
    
    async def get_athlete(self) -> Dict[str, Any]:
        """Get the authenticated athlete's profile."""
        return await self._make_request("GET", "/athlete")
    
    async def list_activities(
        self,
        before: Optional[int] = None,  # Unix timestamp
        after: Optional[int] = None,   # Unix timestamp  
        page: int = 1,
        per_page: int = 30  # Strava recommends 30
    ) -> List[Dict[str, Any]]:
        """
        List athlete activities.
        
        Args:
            before: Activities before this Unix timestamp
            after: Activities after this Unix timestamp
            page: Page number
            per_page: Number of items per page (max 200)
            
        Returns:
            List of activity summaries
        """
        params = {
            "page": page,
            "per_page": min(per_page, 200)  # Strava max is 200
        }
        
        if before is not None:
            params["before"] = before
        if after is not None:
            params["after"] = after
        
        return await self._make_request(
            "GET",
            "/athlete/activities",
            params=params
        )
    
    async def get_activity(self, activity_id: int, include_all_efforts: bool = False) -> Dict[str, Any]:
        """
        Get detailed information about a specific activity.
        
        Args:
            activity_id: The activity ID
            include_all_efforts: Include all segment efforts
            
        Returns:
            Detailed activity data
        """
        params = {}
        if include_all_efforts:
            params["include_all_efforts"] = "true"
        
        return await self._make_request(
            "GET",
            f"/activities/{activity_id}",
            params=params if params else None
        )
    
    async def get_activity_streams(
        self, 
        activity_id: int,
        keys: Optional[List[str]] = None,
        key_by_type: bool = True
    ) -> Dict[str, Any]:
        """
        Get activity streams (time series data).
        
        Args:
            activity_id: The activity ID
            keys: List of stream types to retrieve (e.g., ['time', 'distance', 'altitude', 'heartrate'])
            key_by_type: Return dict keyed by stream type
            
        Returns:
            Stream data
        """
        # Default streams to retrieve
        if keys is None:
            keys = ['time', 'distance', 'altitude', 'velocity_smooth', 'heartrate', 
                   'cadence', 'watts', 'temp', 'moving', 'grade_smooth']
        
        params = {
            "keys": ",".join(keys),
            "key_by_type": str(key_by_type).lower()
        }
        
        return await self._make_request(
            "GET",
            f"/activities/{activity_id}/streams",
            params=params
        )
    
    async def get_all_activities(
        self,
        after: Optional[datetime] = None,
        per_page: int = 30
    ) -> List[Dict[str, Any]]:
        """
        Get all activities, handling pagination automatically.
        
        Args:
            after: Only get activities after this datetime
            per_page: Number of activities per API request
            
        Returns:
            List of all activities
        """
        all_activities = []
        page = 1
        
        # Convert datetime to Unix timestamp if provided
        after_timestamp = int(after.timestamp()) if after else None
        
        while True:
            activities = await self.list_activities(
                after=after_timestamp,
                page=page,
                per_page=per_page
            )
            
            if not activities:
                break
            
            all_activities.extend(activities)
            
            # Strava returns activities in reverse chronological order
            # If we're fetching all activities since a certain date,
            # we can stop when we've gotten them all
            if len(activities) < per_page:
                break
            
            page += 1
        
        return all_activities
    
    async def get_gear(self, gear_id: str) -> Dict[str, Any]:
        """
        Get information about gear (bikes, shoes).
        
        Args:
            gear_id: The gear ID
            
        Returns:
            Gear details
        """
        return await self._make_request("GET", f"/gear/{gear_id}")
    
    async def get_routes(self, athlete_id: Optional[int] = None) -> List[Dict[str, Any]]:
        """
        Get athlete's routes.
        
        Args:
            athlete_id: Athlete ID (defaults to authenticated athlete)
            
        Returns:
            List of routes
        """
        endpoint = f"/athletes/{athlete_id}/routes" if athlete_id else "/athlete/routes"
        return await self._make_request("GET", endpoint)
    
    async def get_stats(self, athlete_id: Optional[int] = None) -> Dict[str, Any]:
        """
        Get athlete statistics.
        
        Args:
            athlete_id: Athlete ID (required, use authenticated athlete's ID)
            
        Returns:
            Athlete statistics
        """
        if not athlete_id:
            # Get authenticated athlete's ID first
            athlete = await self.get_athlete()
            athlete_id = athlete['id']
        
        return await self._make_request("GET", f"/athletes/{athlete_id}/stats")