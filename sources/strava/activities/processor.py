"""
Strava Activities Stream Processor - PostgreSQL Storage
========================================================

This processor handles Strava activity data using the stream storage strategy:
- All activity metrics → PostgreSQL
- No MinIO needed (all data is structured)
"""

from typing import Dict, Any, List, Type
from datetime import datetime
from pydantic import BaseModel
from sources.base.processors.base import StreamProcessor
from .models import StreamStravaActivities


class StravaActivitiesStreamProcessor(StreamProcessor):
    """
    Process Strava activities streams - all data goes to PostgreSQL.
    
    Activity data is all structured metrics, so no MinIO storage needed.
    Configuration is auto-loaded from _stream.yaml.
    """
    
    @property
    def model_class(self) -> Type[BaseModel]:
        """Return the Pydantic model for Strava activities data."""
        return StreamStravaActivities
    
    async def process_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process Strava activity record for PostgreSQL storage.
        
        All fields go directly to PostgreSQL - no MinIO needed.
        
        Args:
            record: Raw Strava activity with structure:
                {
                    "id": 123456789,
                    "name": "Morning Run",
                    "type": "Run",
                    "sport_type": "Run",
                    "distance": 5000.0,
                    "moving_time": 1800,
                    "elapsed_time": 1900,
                    "total_elevation_gain": 50.0,
                    "start_date": "2025-01-01T06:00:00Z",
                    "start_date_local": "2025-01-01T08:00:00",
                    "timezone": "(GMT+02:00) Europe/Paris",
                    "achievement_count": 2,
                    "kudos_count": 10,
                    "comment_count": 3,
                    "athlete_count": 1,
                    "map": {...},
                    "average_speed": 2.78,
                    "max_speed": 3.5,
                    "average_heartrate": 145.2,
                    "max_heartrate": 165,
                    "calories": 450,
                    ...
                }
        
        Returns:
            Processed record ready for PostgreSQL storage
        """
        # Parse timestamps
        start_date = self._parse_timestamp(record.get('start_date'))
        start_date_local = self._parse_timestamp(record.get('start_date_local'))
        
        # Build processed record with all key metrics
        processed = {
            'activity_id': record.get('id'),
            'name': record.get('name'),
            'type': record.get('type'),
            'sport_type': record.get('sport_type'),
            'distance': record.get('distance'),
            'moving_time': record.get('moving_time'),
            'elapsed_time': record.get('elapsed_time'),
            'total_elevation_gain': record.get('total_elevation_gain'),
            'start_date': start_date,
            'start_date_local': start_date_local,
            'timezone': record.get('timezone'),
            'achievement_count': record.get('achievement_count'),
            'kudos_count': record.get('kudos_count'),
            'comment_count': record.get('comment_count'),
            'athlete_count': record.get('athlete_count'),
            'trainer': record.get('trainer', False),
            'commute': record.get('commute', False),
            'manual': record.get('manual', False),
            'private': record.get('private', False),
            'flagged': record.get('flagged', False),
            'workout_type': record.get('workout_type'),
            'gear_id': record.get('gear_id'),
            'average_speed': record.get('average_speed'),
            'max_speed': record.get('max_speed'),
            'average_cadence': record.get('average_cadence'),
            'average_watts': record.get('average_watts'),
            'max_watts': record.get('max_watts'),
            'weighted_average_watts': record.get('weighted_average_watts'),
            'kilojoules': record.get('kilojoules'),
            'device_watts': record.get('device_watts'),
            'average_heartrate': record.get('average_heartrate'),
            'max_heartrate': record.get('max_heartrate'),
            'calories': record.get('calories'),
            'suffer_score': record.get('suffer_score'),
            'elev_high': record.get('elev_high'),
            'elev_low': record.get('elev_low'),
        }
        
        # Add timestamp for time-series queries
        processed['timestamp'] = start_date
        
        # Store complex objects as JSONB
        if record.get('map'):
            processed['map'] = record['map']
        if record.get('splits_metric'):
            processed['splits_metric'] = record['splits_metric']
        if record.get('splits_standard'):
            processed['splits_standard'] = record['splits_standard']
        if record.get('segment_efforts'):
            processed['segment_efforts'] = record['segment_efforts']
        if record.get('laps'):
            processed['laps'] = record['laps']
        if record.get('best_efforts'):
            processed['best_efforts'] = record['best_efforts']
        if record.get('photos'):
            processed['photos'] = record['photos']
        
        # Store any additional stats
        if record.get('stats'):
            processed['stats'] = record['stats']
        
        # Store the full activity data for reference
        processed['full_activity'] = record
        
        return processed
    
    def _parse_timestamp(self, timestamp_str: str) -> datetime:
        """Parse ISO format timestamp string."""
        if not timestamp_str:
            return None
        
        from datetime import datetime
        
        try:
            # Handle Strava's ISO format (with or without Z)
            if timestamp_str.endswith('Z'):
                return datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
            else:
                # Assume local time if no Z
                return datetime.fromisoformat(timestamp_str)
        except:
            return None