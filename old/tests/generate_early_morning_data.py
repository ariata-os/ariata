#!/usr/bin/env python3
"""
Generate early morning sleep data (00:00 - 07:23 CT) for July 1, 2025.
This fills the gap before the person wakes up at 07:23 AM.
"""

import json
import random
import math
from datetime import datetime, timedelta
from zoneinfo import ZoneInfo
from typing import List, Dict
import uuid


def generate_sleep_heart_rate(timestamp: datetime) -> float:
    """Generate realistic heart rate during sleep."""
    # Base heart rate 55-65 bpm
    base_hr = 60
    
    # Add some variation based on sleep stage (simplified)
    hour = timestamp.hour
    if 2 <= hour < 4:  # Deep sleep
        base_hr -= 5
    elif 4 <= hour < 6:  # REM sleep
        base_hr += 3
    
    # Add small random variation
    variation = random.gauss(0, 2)
    return max(50, min(70, base_hr + variation))


def generate_sleep_hrv(timestamp: datetime) -> float:
    """Generate realistic HRV during sleep."""
    # Base HRV 40-60 ms
    base_hrv = 50
    
    # HRV typically higher during deep sleep
    hour = timestamp.hour
    if 2 <= hour < 4:  # Deep sleep
        base_hrv += 10
    
    # Add variation
    variation = random.gauss(0, 5)
    return max(30, min(80, base_hrv + variation))


def generate_audio_level(timestamp: datetime) -> float:
    """Generate realistic audio levels during sleep."""
    # Very quiet: -50 to -45 dB
    base_level = -48
    
    # Occasional small noises
    if random.random() < 0.05:  # 5% chance of noise
        base_level += random.uniform(3, 8)  # Snoring, movement
    
    # Add small variation
    variation = random.gauss(0, 1)
    return max(-55, min(-40, base_level + variation))


def generate_early_morning_data():
    """Generate all early morning data from midnight to 07:23 CT."""
    
    # Define time range in Central Time
    ct_zone = ZoneInfo("America/Chicago")
    utc_zone = ZoneInfo("UTC")
    
    # Start at midnight CT on July 1, 2025
    start_ct = datetime(2025, 7, 1, 0, 0, 0, tzinfo=ct_zone)
    # End at 07:23:14 CT (when person wakes up)
    end_ct = datetime(2025, 7, 1, 7, 23, 14, tzinfo=ct_zone)
    
    # Convert to UTC for storage
    start_utc = start_ct.astimezone(utc_zone)
    end_utc = end_ct.astimezone(utc_zone)
    
    # Home location (from TEST_DAY.md)
    home_lat = 36.1744
    home_lng = -86.7444
    home_altitude = 165
    
    # Generate data points every 5 minutes
    all_data = {
        "ios_location": [],
        "ios_healthkit": [],
        "ios_audio": []
    }
    
    current_time = start_ct
    while current_time < end_ct:
        # Convert current time to UTC for storage
        current_utc = current_time.astimezone(utc_zone)
        timestamp_str = current_utc.strftime("%Y-%m-%dT%H:%M:%SZ")
        
        # Generate location data (stationary at home)
        location_point = {
            "timestamp": timestamp_str,
            "latitude": home_lat + random.gauss(0, 0.00001),  # Tiny variation
            "longitude": home_lng + random.gauss(0, 0.00001),
            "altitude": home_altitude + random.gauss(0, 0.1),
            "horizontal_accuracy": 5.0,
            "vertical_accuracy": 3.0,
            "speed": 0.0,  # Not moving
            "course": 0.0,
            "floor": 2
        }
        all_data["ios_location"].append(location_point)
        
        # Generate health data
        heart_rate = generate_sleep_heart_rate(current_time)
        hrv = generate_sleep_hrv(current_time)
        
        health_point = {
            "timestamp": timestamp_str,
            "heart_rate": round(heart_rate, 1),
            "hrv": round(hrv, 1),
            "activity_type": "sleeping",
            "confidence": 0.95
        }
        all_data["ios_healthkit"].append(health_point)
        
        # Generate audio data (every 15 minutes)
        if current_time.minute % 15 == 0:
            audio_level = generate_audio_level(current_time)
            audio_point = {
                "timestamp": timestamp_str,
                "audio_level": round(audio_level, 1),
                "peak_level": round(audio_level + random.uniform(0, 2), 1),
                "duration_seconds": 900  # 15 minute chunks
            }
            all_data["ios_audio"].append(audio_point)
        
        # Move to next time point (5 minutes)
        current_time += timedelta(minutes=5)
    
    return all_data


def merge_with_existing_data(early_data: Dict, existing_file: str) -> Dict:
    """Merge early morning data with existing test data."""
    
    # Load existing data
    with open(existing_file, 'r') as f:
        existing = json.load(f)
    
    # Merge location data
    if "data" in existing:
        # Prepend early morning location data
        all_location = early_data["ios_location"] + existing["data"]
        # Sort by timestamp
        all_location.sort(key=lambda x: x["timestamp"])
        existing["data"] = all_location
        
        # Update metadata if it exists
        if "batch_metadata" not in existing:
            existing["batch_metadata"] = {}
        existing["batch_metadata"]["start_time"] = all_location[0]["timestamp"]
        existing["batch_metadata"]["end_time"] = all_location[-1]["timestamp"]
        existing["batch_metadata"]["total_points"] = len(all_location)
    
    return existing


def main():
    """Generate and save early morning test data."""
    
    print("Generating early morning sleep data (00:00 - 07:23 CT)...")
    early_data = generate_early_morning_data()
    
    print(f"Generated:")
    print(f"  - {len(early_data['ios_location'])} location points")
    print(f"  - {len(early_data['ios_healthkit'])} health points")
    print(f"  - {len(early_data['ios_audio'])} audio points")
    
    # Merge with existing location data
    print("\nMerging with existing location data...")
    merged_location = merge_with_existing_data(
        early_data, 
        'test_data_ios_location.json'
    )
    
    # Save updated location data
    with open('test_data_ios_location.json', 'w') as f:
        json.dump(merged_location, f, indent=2)
    print("✅ Updated test_data_ios_location.json")
    
    # Save health data separately (new file)
    health_data = {
        "device_id": "dev_iphone_001",
        "stream_name": "ios_healthkit",
        "data": early_data["ios_healthkit"],
        "batch_metadata": {
            "total_points": len(early_data["ios_healthkit"]),
            "date": "2025-07-01",
            "start_time": early_data["ios_healthkit"][0]["timestamp"],
            "end_time": early_data["ios_healthkit"][-1]["timestamp"]
        }
    }
    
    with open('test_data_ios_healthkit.json', 'w') as f:
        json.dump(health_data, f, indent=2)
    print("✅ Created test_data_ios_healthkit.json")
    
    # Create audio data in the same format as existing
    print("\nCreating early morning audio data...")
    audio_chunks = []
    for i, point in enumerate(early_data["ios_audio"]):
        # Convert to the expected format for ios_mic stream
        # Parse the timestamp to add 30 seconds for end time
        from datetime import datetime as dt2, timedelta as td2
        start_dt = dt2.fromisoformat(point["timestamp"].replace("Z", "+00:00"))
        end_dt = start_dt + td2(seconds=30)
        
        chunk = {
            "id": f"early-{i:04d}",
            "audio_data": "UklGRlYAAABXQVZFZm10IBAAAAABAAEA9AEAAPQBAAABAAgAZGF0YTIAAAB/f3+AgH+AgH+AgH9/gH9/f39/gH9/gH9/f3+AgH+Af39/f39/f4CAf4B/gIB/gH+AgA==",  # Dummy quiet audio
            "timestamp_start": point["timestamp"],
            "timestamp_end": end_dt.strftime("%Y-%m-%dT%H:%M:%SZ"),
            "duration": 30000,  # 30 seconds
            "overlap_duration": 2,
            "audio_format": "wav",
            "sample_rate": 500,
            "audio_level": point["audio_level"],  # Add our computed level
            "peak_level": point["peak_level"]
        }
        audio_chunks.append(chunk)
    
    # Load and update existing audio file
    try:
        with open('test_data_ios_audio.json', 'r') as f:
            existing_audio = json.load(f)
        
        # Prepend early morning chunks
        all_chunks = audio_chunks + existing_audio.get("data", [])
        existing_audio["data"] = all_chunks
        
        if "batch_metadata" not in existing_audio:
            existing_audio["batch_metadata"] = {}
        existing_audio["batch_metadata"]["total_records"] = len(all_chunks)
        
        with open('test_data_ios_audio.json', 'w') as f:
            json.dump(existing_audio, f, indent=2)
        print(f"✅ Updated test_data_ios_audio.json (added {len(audio_chunks)} early morning chunks)")
    except FileNotFoundError:
        print("⚠️  test_data_ios_audio.json not found, skipping audio update")
    
    print("\n✨ Early morning data generation complete!")
    print("The test data now covers the full day from 00:00 to 23:59 CT")


if __name__ == "__main__":
    main()