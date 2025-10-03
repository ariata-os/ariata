#!/usr/bin/env python3
"""
Generate realistic iOS Core Location test data for Nashville, TN.
Based on TEST_DAY.md schedule for July 1, 2025.
"""

import json
import random
import math
from datetime import datetime, timedelta, timezone
from zoneinfo import ZoneInfo
from typing import List, Dict, Tuple, Optional
from dataclasses import dataclass
import uuid


@dataclass
class Location:
    """Represents a physical location."""
    name: str
    lat: float
    lng: float
    altitude: float
    indoor: bool


@dataclass 
class Event:
    """Represents an event from the schedule."""
    time_str: str  # Time in CDT
    location_name: str
    activity: str
    duration_minutes: int
    mode: Optional[str] = None  # Transport mode TO this location
    location: Optional[Location] = None
    start_time: Optional[datetime] = None


# Define all locations from TEST_DAY.md
LOCATIONS = {
    "home": Location("Home - East Nashville", 36.1744, -86.7444, 165, True),
    "shelby_bottoms": Location("Shelby Bottoms Greenway", 36.1780, -86.7380, 130, False),
    "crema": Location("Crema Coffee", 36.1516, -86.7960, 175, True),
    "gym": Location("The Gym Nashville", 36.1486, -86.8050, 180, True),
    "farmers_market": Location("Nashville Farmers' Market", 36.1688, -86.7847, 155, False),
    "centennial": Location("Centennial Park", 36.1490, -86.8127, 160, False),
    "cvs": Location("CVS Green Hills", 36.1065, -86.8163, 185, True),
    "shell": Location("Shell Station", 36.1650, -86.7800, 170, False),
    "etch": Location("Etch Restaurant", 36.1644, -86.7819, 172, True),
    "patterson": Location("The Patterson House", 36.1550, -86.7965, 168, True),
}

# Parse schedule from TEST_DAY.md
SCHEDULE = [
    Event("07:23:14", "home", "wake", 22),
    Event("07:45:32", "shelby_bottoms", "dog_walk", 27, "walking"),
    Event("08:12:27", "home", "breakfast", 40, "walking"),
    Event("08:52:09", "crema", "coffee_work", 83, "biking"),
    Event("10:15:21", "gym", "workout", 75, "driving"),
    Event("11:30:44", "farmers_market", "shopping", 43, "driving"),
    Event("12:14:07", "home", "lunch", 80, "driving"),
    Event("13:34:28", "centennial", "pickleball", 71, "driving"),
    Event("14:45:19", "home", "dog_walk_2", 47, "driving"),
    Event("15:32:08", "cvs", "errand", 28, "driving"),
    Event("16:00:33", "home", "video_call", 45, "driving"),
    Event("16:45:12", "home", "text", 13),
    Event("16:58:42", "shell", "gas", 4, "driving"),
    Event("17:02:18", "etch", "dinner", 118, "driving"),
    Event("19:00:42", "home", "dog_walk_3", 87, "driving"),
    Event("20:28:03", "patterson", "drinks", 79, "driving"),
    Event("21:47:29", "patterson", "quiz", 43),
    Event("22:30:15", "home", "reading", 68, "driving"),
    Event("23:38:21", "home", "sleep", 0),
]


class LocationDataGenerator:
    def __init__(self):
        self.device_id = str(uuid.uuid4())
        self.base_date = datetime(2025, 7, 1, 0, 0, 0, tzinfo=ZoneInfo('America/Chicago'))
        self.data_points = []
        
        # Initialize schedule with locations and times
        for event in SCHEDULE:
            event.location = LOCATIONS.get(event.location_name)
            time_parts = event.time_str.split(':')
            event.start_time = self.base_date.replace(
                hour=int(time_parts[0]),
                minute=int(time_parts[1]),
                second=int(time_parts[2])
            )
    
    def generate_day_data(self) -> Dict:
        """Generate a full day of location data."""
        current_location = LOCATIONS["home"]
        current_time = SCHEDULE[0].start_time
        
        for i, event in enumerate(SCHEDULE):
            # For transitions, we need to leave early enough to arrive on time
            if event.mode and event.location != current_location:
                # Estimate travel time
                travel_time = self.estimate_travel_time(
                    current_location, event.location, event.mode
                )
                departure_time = event.start_time - timedelta(minutes=travel_time)
                
                # If departure is before current time, we need to cut the previous event short
                if departure_time < current_time:
                    # Leave immediately from current time
                    departure_time = current_time
                    # Extend arrival slightly if needed to have some transition time
                    if (event.start_time - departure_time).total_seconds() < 60:
                        departure_time = event.start_time - timedelta(minutes=1)
                
                # Generate transition
                transition_points = self.generate_transition(
                    current_location, event.location,
                    departure_time, event.start_time,
                    event.mode
                )
                self.data_points.extend(transition_points)
                current_time = event.start_time
            
            # Generate data for the event itself
            if event.duration_minutes > 0:
                # Calculate actual event duration
                event_duration = event.duration_minutes
                
                # Check if we need to leave early for the next event
                if i < len(SCHEDULE) - 1:
                    next_event = SCHEDULE[i + 1]
                    if next_event.mode and next_event.location != event.location:
                        # Calculate when we need to leave
                        travel_time = self.estimate_travel_time(
                            event.location, next_event.location, next_event.mode
                        )
                        must_leave_by = next_event.start_time - timedelta(minutes=travel_time)
                        event_end = current_time + timedelta(minutes=event_duration)
                        
                        # If event would run too long, shorten it
                        if event_end > must_leave_by:
                            max_duration = (must_leave_by - current_time).total_seconds() / 60
                            event_duration = max(1, int(max_duration))
                
                # Generate event points with adjusted duration
                event_points = self.generate_event_with_duration(
                    event, current_time, event_duration
                )
                self.data_points.extend(event_points)
                current_time = current_time + timedelta(minutes=event_duration)
            
            current_location = event.location
        
        return {
            "stream_name": "ios_location",
            "device_id": self.device_id,
            "data": self.data_points
        }
    
    def generate_event_with_duration(self, event: Event, start_time: datetime, duration_minutes: int) -> List[Dict]:
        """Generate location points for an event with specific duration."""
        # Temporarily override the event duration
        original_duration = event.duration_minutes
        event.duration_minutes = duration_minutes
        points = self.generate_event(event, start_time)
        event.duration_minutes = original_duration
        return points
    
    def estimate_travel_time(self, from_loc: Location, to_loc: Location, mode: str) -> float:
        """Estimate travel time in minutes between locations."""
        # Calculate distance (rough estimate)
        lat_diff = abs(to_loc.lat - from_loc.lat)
        lng_diff = abs(to_loc.lng - from_loc.lng)
        distance_km = math.sqrt((lat_diff * 111)**2 + (lng_diff * 111)**2)
        
        # Estimate time based on mode
        if mode == "walking":
            return distance_km * 12  # ~5 km/h walking
        elif mode == "biking":
            return distance_km * 4   # ~15 km/h biking
        elif mode == "driving":
            return distance_km * 2 + 3  # ~30 km/h city driving + parking
        return 5  # Default
    
    def generate_event(self, event: Event, start_time: datetime) -> List[Dict]:
        """Generate location points for a stationary or activity event."""
        points = []
        
        # Determine event type and generate appropriate data
        if "walk" in event.activity:
            points = self.generate_walking_event(event, start_time)
        elif event.activity in ["workout", "pickleball"]:
            points = self.generate_active_event(event, start_time)
        elif event.activity == "shopping":
            points = self.generate_shopping_event(event, start_time)
        else:
            points = self.generate_stationary_event(event, start_time)
        
        return points
    
    def generate_stationary_event(self, event: Event, start_time: datetime) -> List[Dict]:
        """Generate data for stationary activities (home, restaurant, coffee shop)."""
        points = []
        num_points = event.duration_minutes * 6  # One point every 10 seconds
        
        for i in range(num_points):
            timestamp = start_time + timedelta(seconds=i * 10)
            
            # GPS drift for stationary position
            if event.location.indoor:
                # Indoor has more drift
                drift = 0.00005  # ~5 meters
                accuracy = random.uniform(10, 25)
            else:
                # Outdoor has less drift
                drift = 0.00002  # ~2 meters
                accuracy = random.uniform(5, 10)
            
            lat = event.location.lat + random.uniform(-drift, drift)
            lng = event.location.lng + random.uniform(-drift, drift)
            
            points.append(self.create_location_point(
                lat, lng, event.location.altitude, 0, timestamp, accuracy
            ))
        
        return points
    
    def generate_walking_event(self, event: Event, start_time: datetime) -> List[Dict]:
        """Generate realistic walking patterns."""
        points = []
        num_points = event.duration_minutes * 6
        
        # Create a walking loop pattern
        base_lat = event.location.lat
        base_lng = event.location.lng
        
        for i in range(num_points):
            timestamp = start_time + timedelta(seconds=i * 10)
            progress = i / num_points
            
            # Create a loop path
            angle = progress * 2 * math.pi
            radius = 0.003  # About 300m radius
            
            # Add variation to make it look natural
            radius_var = radius * (1 + 0.3 * math.sin(angle * 5))
            
            lat = base_lat + radius_var * math.cos(angle)
            lng = base_lng + radius_var * math.sin(angle) * 1.2  # Elliptical
            
            # Walking speed varies
            if random.random() < 0.1:  # 10% stops (dog sniffing, etc)
                speed = 0
            else:
                speed = random.uniform(1.0, 1.8)  # 1.0-1.8 m/s walking
            
            # Add small random drift
            lat += random.uniform(-0.00001, 0.00001)
            lng += random.uniform(-0.00001, 0.00001)
            
            points.append(self.create_location_point(
                lat, lng, event.location.altitude, speed, timestamp,
                random.uniform(5, 10)
            ))
        
        return points
    
    def generate_active_event(self, event: Event, start_time: datetime) -> List[Dict]:
        """Generate data for active events like workout or pickleball."""
        points = []
        num_points = event.duration_minutes * 6
        
        for i in range(num_points):
            timestamp = start_time + timedelta(seconds=i * 10)
            
            # Small movements within the venue
            if random.random() < 0.3:  # 30% of time moving
                drift = 0.0001  # ~10m movements
                speed = random.uniform(0.5, 2.0)
            else:
                drift = 0.00002
                speed = 0
            
            lat = event.location.lat + random.uniform(-drift, drift)
            lng = event.location.lng + random.uniform(-drift, drift)
            
            accuracy = random.uniform(8, 20) if event.location.indoor else random.uniform(5, 10)
            
            points.append(self.create_location_point(
                lat, lng, event.location.altitude, speed, timestamp, accuracy
            ))
        
        return points
    
    def generate_shopping_event(self, event: Event, start_time: datetime) -> List[Dict]:
        """Generate data for shopping/market visits."""
        points = []
        num_points = event.duration_minutes * 6
        
        for i in range(num_points):
            timestamp = start_time + timedelta(seconds=i * 10)
            progress = i / num_points
            
            # Simulate walking between market stalls
            if random.random() < 0.4:  # 40% of time walking
                # Random walk pattern
                drift = 0.0002  # ~20m movements
                speed = random.uniform(0.8, 1.5)
            else:
                drift = 0.00005
                speed = 0
            
            lat = event.location.lat + random.uniform(-drift, drift)
            lng = event.location.lng + random.uniform(-drift, drift)
            
            points.append(self.create_location_point(
                lat, lng, event.location.altitude, speed, timestamp,
                random.uniform(6, 12)
            ))
        
        return points
    
    def generate_transition(self, from_loc: Location, to_loc: Location,
                          start_time: datetime, arrival_time: datetime,
                          mode: str) -> List[Dict]:
        """Generate smooth transition between locations."""
        points = []
        
        # Calculate travel duration
        duration = (arrival_time - start_time).total_seconds()
        num_points = int(duration / 10)  # One point every 10 seconds
        
        if num_points <= 0:
            return points
        
        for i in range(num_points):
            timestamp = start_time + timedelta(seconds=i * 10)
            progress = i / (num_points - 1) if num_points > 1 else 1
            
            # Smooth S-curve interpolation for more natural movement
            smooth_progress = self.smooth_step(progress)
            
            # Interpolate position
            lat = from_loc.lat + (to_loc.lat - from_loc.lat) * smooth_progress
            lng = from_loc.lng + (to_loc.lng - from_loc.lng) * smooth_progress
            altitude = from_loc.altitude + (to_loc.altitude - from_loc.altitude) * smooth_progress
            
            # Calculate speed based on mode and position in journey
            if mode == "walking":
                base_speed = 1.4  # m/s
                speed = base_speed * (1 + 0.2 * math.sin(progress * math.pi))
            elif mode == "biking":
                base_speed = 5.0  # m/s
                # Acceleration at start, deceleration at end
                if progress < 0.1:
                    speed = base_speed * (progress * 10)
                elif progress > 0.9:
                    speed = base_speed * ((1 - progress) * 10)
                else:
                    speed = base_speed + random.uniform(-1, 1)
            elif mode == "driving":
                base_speed = 12.0  # m/s (~27 mph city driving)
                # Stop lights and traffic
                if random.random() < 0.15:  # 15% chance of stop
                    speed = 0
                elif progress < 0.05:  # Accelerating from stop
                    speed = base_speed * (progress * 20)
                elif progress > 0.95:  # Slowing to stop
                    speed = base_speed * ((1 - progress) * 20)
                else:
                    speed = base_speed + random.uniform(-3, 3)
            else:
                speed = 0
            
            # Add realistic path variation
            if mode != "walking":  # Walking already has variation
                lat += random.uniform(-0.00002, 0.00002)
                lng += random.uniform(-0.00002, 0.00002)
            
            accuracy = random.uniform(5, 15) if mode == "driving" else random.uniform(5, 10)
            
            points.append(self.create_location_point(
                lat, lng, altitude, max(0, speed), timestamp, accuracy
            ))
        
        return points
    
    def smooth_step(self, t: float) -> float:
        """Smooth S-curve interpolation."""
        return t * t * (3 - 2 * t)
    
    def create_location_point(self, lat: float, lng: float, altitude: float,
                             speed: float, timestamp: datetime,
                             horizontal_accuracy: float) -> Dict:
        """Create a single location data point."""
        return {
            "latitude": round(lat, 8),
            "longitude": round(lng, 8),
            "altitude": round(altitude + random.uniform(-2, 2), 1),
            "speed": round(max(0, speed), 2),
            "horizontal_accuracy": round(horizontal_accuracy, 2),
            "vertical_accuracy": round(horizontal_accuracy * 1.5, 2),
            "timestamp": timestamp.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        }


def main():
    """Generate and save the location test data."""
    generator = LocationDataGenerator()
    
    print("Generating Nashville location data (v2)...")
    data = generator.generate_day_data()
    
    # Validate data
    timestamps = [p['timestamp'] for p in data['data']]
    if len(timestamps) != len(set(timestamps)):
        print("WARNING: Duplicate timestamps detected!")
    
    # Save to file
    output_file = 'test_data_ios_location.json'
    with open(output_file, 'w') as f:
        json.dump(data, f, indent=2)
    
    print(f"Generated {len(data['data'])} location points")
    print(f"Saved to {output_file}")
    
    # Print summary
    print("\nSummary:")
    print(f"Device ID: {data['device_id']}")
    print(f"First timestamp: {data['data'][0]['timestamp']}")
    print(f"Last timestamp: {data['data'][-1]['timestamp']}")
    
    speeds = [p['speed'] for p in data['data']]
    non_zero = [s for s in speeds if s > 0]
    if non_zero:
        print(f"Average speed (when moving): {sum(non_zero)/len(non_zero):.2f} m/s")
    print(f"Percentage stationary: {(len(speeds) - len(non_zero)) / len(speeds) * 100:.1f}%")


if __name__ == "__main__":
    main()