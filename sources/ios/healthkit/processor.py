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
        start_date = self._parse_timestamp(record.get('startDate'))
        end_date = self._parse_timestamp(record.get('endDate'))
        
        # Determine the type of health data
        sample_type = record.get('type', '')
        
        # Build processed record based on the type
        processed = {
            'sample_type': sample_type,
            'timestamp': timestamp,
            'start_date': start_date,
            'end_date': end_date,
            'source_name': record.get('sourceName'),
            'source_version': record.get('sourceVersion'),
        }
        
        # Process based on sample type
        if 'HeartRate' in sample_type:
            processed['heart_rate'] = record.get('value')
            processed['unit'] = record.get('unit', 'count/min')
        
        elif 'HeartRateVariability' in sample_type:
            processed['hrv'] = record.get('value')
            processed['unit'] = record.get('unit', 'ms')
        
        elif 'StepCount' in sample_type:
            processed['steps'] = int(record.get('value', 0))
            processed['unit'] = record.get('unit', 'count')
        
        elif 'ActiveEnergyBurned' in sample_type:
            processed['active_energy'] = record.get('value')
            processed['unit'] = record.get('unit', 'kcal')
        
        elif 'DistanceWalkingRunning' in sample_type:
            processed['distance'] = record.get('value')
            processed['unit'] = record.get('unit', 'm')
        
        elif 'AppleExerciseTime' in sample_type:
            processed['exercise_time'] = record.get('value')
            processed['unit'] = record.get('unit', 'min')
        
        elif 'SleepAnalysis' in sample_type:
            processed['sleep_state'] = record.get('value')  # 0=InBed, 1=Asleep, 2=Awake
            processed['sleep_source'] = record.get('sourceName')
        
        elif 'Workout' in sample_type:
            # Handle workout data
            processed['workout_type'] = sample_type
            processed['duration'] = record.get('duration')
            processed['total_distance'] = record.get('totalDistance')
            processed['total_energy_burned'] = record.get('totalEnergyBurned')
            processed['average_heart_rate'] = record.get('averageHeartRate')
            processed['max_heart_rate'] = record.get('maxHeartRate')
            processed['elevation_ascended'] = record.get('elevationAscended')
            processed['elevation_descended'] = record.get('elevationDescended')
            processed['indoor_workout'] = record.get('indoorWorkout', False)
            
            # Store workout events and samples
            if record.get('workoutEvents'):
                processed['workout_events'] = record['workoutEvents']
            if record.get('workoutRoute'):
                processed['workout_route'] = record['workoutRoute']
        
        elif 'RespiratoryRate' in sample_type:
            processed['respiratory_rate'] = record.get('value')
            processed['unit'] = record.get('unit', 'count/min')
        
        elif 'OxygenSaturation' in sample_type:
            processed['oxygen_saturation'] = record.get('value')
            processed['unit'] = record.get('unit', '%')
        
        elif 'BodyTemperature' in sample_type:
            processed['body_temperature'] = record.get('value')
            processed['unit'] = record.get('unit', 'degC')
        
        elif 'BloodPressure' in sample_type:
            processed['systolic_pressure'] = record.get('systolic')
            processed['diastolic_pressure'] = record.get('diastolic')
            processed['unit'] = record.get('unit', 'mmHg')
        
        else:
            # Generic handling for other types
            processed['value'] = record.get('value')
            processed['unit'] = record.get('unit')
        
        # Add device information if available
        if record.get('device'):
            processed['device'] = record['device']
        
        # Add metadata if available
        if record.get('metadata'):
            processed['metadata'] = record['metadata']
        
        # Store any additional fields in raw_data
        extra_fields = set(record.keys()) - {'type', 'value', 'unit', 'timestamp', 
                                              'startDate', 'endDate', 'sourceName', 
                                              'sourceVersion', 'device', 'metadata'}
        if extra_fields:
            processed['raw_data'] = {k: record[k] for k in extra_fields}
        
        return processed
    
    def _parse_timestamp(self, timestamp_str: str) -> datetime:
        """Parse ISO format timestamp string."""
        if not timestamp_str:
            return None
        
        from datetime import datetime
        
        try:
            # Handle ISO format with Z
            if timestamp_str.endswith('Z'):
                return datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
            else:
                return datetime.fromisoformat(timestamp_str)
        except:
            return None