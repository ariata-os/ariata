"""
iOS HealthKit Stream Processor - PostgreSQL Storage
====================================================

This processor handles iOS HealthKit data using the stream storage strategy:
- All health metrics → PostgreSQL
- No MinIO needed (all data is structured)
"""

from typing import Dict, Any, List, Type
from datetime import datetime
from pydantic import BaseModel

from sources.base.processors.base import StreamProcessor
from .models import StreamIosHealthkit


class IosHealthkitStreamProcessor(StreamProcessor):
    """
    Process iOS HealthKit streams - all data goes to PostgreSQL.

    Health metrics are all structured data, so no MinIO storage needed.
    Configuration is auto-loaded from _stream.yaml.
    """

    @property
    def model_class(self) -> Type[BaseModel]:
        """Return the Pydantic model for iOS HealthKit data."""
        return StreamIosHealthkit

    async def process_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process iOS HealthKit record for PostgreSQL storage.

        All fields go directly to PostgreSQL - no MinIO needed.

        Args:
            record: Raw iOS HealthKit sample with structure:
                {
                    "type": "HKQuantityTypeIdentifierHeartRate",
                    "value": 72.0,
                    "unit": "count/min",
                    "timestamp": "2025-01-01T12:00:00Z",
                    "startDate": "2025-01-01T12:00:00Z",
                    "endDate": "2025-01-01T12:00:00Z",
                    "sourceName": "Apple Watch",
                    "sourceVersion": "10.1",
                    "device": {...},
                    "metadata": {...}
                }

                Or for workouts:
                {
                    "type": "HKWorkoutActivityTypeRunning",
                    "duration": 1800,
                    "totalDistance": 5000,
                    "totalEnergyBurned": 450,
                    "startDate": "2025-01-01T06:00:00Z",
                    "endDate": "2025-01-01T06:30:00Z",
                    ...
                }

        Returns:
            Processed record ready for PostgreSQL storage
        """
        # Parse timestamps
        timestamp = self._parse_timestamp(record.get('timestamp') or record.get('startDate'))

        # Determine the type of health data
        sample_type = record.get('type', '')

        # Build processed record - only include fields that exist in the schema
        processed = {
            'timestamp': timestamp,
            'device_name': record.get('sourceName'),
        }

        # Store the original type in raw_data for reference
        raw_data = {
            'type': sample_type,
            'unit': record.get('unit'),
            'metadata': record.get('metadata', {}),
            'device': record.get('device', {}),
            'sourceVersion': record.get('sourceVersion'),
            'startDate': record.get('startDate'),
            'endDate': record.get('endDate')
        }

        # Process based on sample type - only map to columns that exist in schema
        if 'HeartRate' in sample_type:
            processed['heart_rate'] = record.get('value')

        elif 'HeartRateVariability' in sample_type:
            processed['hrv'] = record.get('value')

        elif 'StepCount' in sample_type:
            processed['steps'] = int(record.get('value', 0))

        elif 'ActiveEnergyBurned' in sample_type:
            processed['active_energy'] = record.get('value')

        elif 'SleepAnalysis' in sample_type:
            # Map sleep state to sleep_stage field
            sleep_value = record.get('value')
            if sleep_value == 0:
                processed['sleep_stage'] = 'InBed'
            elif sleep_value == 1:
                processed['sleep_stage'] = 'Asleep'
            elif sleep_value == 2:
                processed['sleep_stage'] = 'Awake'
            else:
                processed['sleep_stage'] = str(sleep_value)

        elif 'Workout' in sample_type:
            # Handle workout data - only workout_type and workout_duration are in schema
            processed['workout_type'] = sample_type.replace('HKWorkoutActivityType', '')
            if record.get('duration'):
                processed['workout_duration'] = int(record.get('duration'))

            # Store extra workout data in raw_data
            raw_data.update({
                'totalDistance': record.get('totalDistance'),
                'totalEnergyBurned': record.get('totalEnergyBurned'),
                'averageHeartRate': record.get('averageHeartRate'),
                'maxHeartRate': record.get('maxHeartRate'),
                'elevationAscended': record.get('elevationAscended'),
                'elevationDescended': record.get('elevationDescended'),
                'indoorWorkout': record.get('indoorWorkout', False),
                'workoutEvents': record.get('workoutEvents'),
                'workoutRoute': record.get('workoutRoute')
            })

        elif 'Walking' in sample_type or 'Running' in sample_type:
            # Activity types
            if 'Walking' in sample_type:
                processed['activity_type'] = 'walking'
            elif 'Running' in sample_type:
                processed['activity_type'] = 'running'

            # Store value in raw_data since these don't map to specific columns
            raw_data['value'] = record.get('value')

        else:
            # For any other types, store all data in raw_data
            raw_data['value'] = record.get('value')

        # Add confidence if present
        if 'confidence' in record:
            processed['confidence'] = record.get('confidence')

        # Always add raw_data to preserve original information
        processed['raw_data'] = raw_data

        # Remove None values to avoid database errors
        processed = {k: v for k, v in processed.items() if v is not None}

        return processed

    def _parse_timestamp(self, timestamp_str: str) -> datetime:
        """Parse ISO format timestamp string."""
        if not timestamp_str:
            return None

        from datetime import datetime

        # Handle different formats
        formats = [
            '%Y-%m-%dT%H:%M:%SZ',
            '%Y-%m-%dT%H:%M:%S.%fZ',
            '%Y-%m-%d %H:%M:%S',
        ]

        for fmt in formats:
            try:
                return datetime.strptime(timestamp_str, fmt)
            except ValueError:
                continue

        # If all formats fail, try iso format
        try:
            return datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
        except:
            return None
