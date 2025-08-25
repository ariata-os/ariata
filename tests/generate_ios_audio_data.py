#!/usr/bin/env python3
"""
Generate minimal iOS mic audio test data for Nashville, TN.
Based on TEST_DAY.md schedule for July 1, 2025.
Creates ultra-small audio clips (0.1 sec) to keep file size minimal.
"""

import json
import gzip
import random
import base64
import struct
from datetime import datetime, timedelta, timezone
from zoneinfo import ZoneInfo
from typing import List, Dict, Optional
from dataclasses import dataclass
import uuid
import numpy as np


@dataclass
class Event:
    """Represents an event from the schedule."""
    time_str: str  # Time in CDT
    location_name: str
    activity: str
    duration_minutes: int
    start_time: Optional[datetime] = None


# Activity-based dB levels (realistic ambient noise levels)
ACTIVITY_DB_LEVELS = {
    "wake": -45,          # Quiet morning
    "dog_walk": -32,      # Outdoor ambient
    "breakfast": -38,     # Kitchen sounds
    "coffee_work": -30,   # Coffee shop buzz
    "workout": -28,       # Gym noise
    "shopping": -26,      # Market chatter
    "lunch": -35,         # Home cooking
    "pickleball": -25,    # Outdoor sports
    "dog_walk_2": -32,    # Afternoon walk
    "errand": -30,        # Store ambience
    "video_call": -35,    # Computer audio
    "text": -40,          # Quiet typing
    "gas": -28,           # Outdoor traffic
    "dinner": -24,        # Restaurant busy
    "dog_walk_3": -33,    # Evening walk
    "drinks": -22,        # Loud bar
    "quiz": -20,          # Pub at peak
    "reading": -42,       # Quiet evening
    "sleep": -50          # Near silence
}

# Parse schedule from TEST_DAY.md
SCHEDULE = [
    Event("07:23:14", "home", "wake", 22),
    Event("07:45:32", "shelby_bottoms", "dog_walk", 27),
    Event("08:12:27", "home", "breakfast", 40),
    Event("08:52:09", "crema", "coffee_work", 83),
    Event("10:15:21", "gym", "workout", 75),
    Event("11:30:44", "farmers_market", "shopping", 43),
    Event("12:14:07", "home", "lunch", 80),
    Event("13:34:28", "centennial", "pickleball", 71),
    Event("14:45:19", "home", "dog_walk_2", 47),
    Event("15:32:08", "cvs", "errand", 28),
    Event("16:00:33", "home", "video_call", 45),
    Event("16:45:12", "home", "text", 13),
    Event("16:58:42", "shell", "gas", 4),
    Event("17:02:18", "etch", "dinner", 118),
    Event("19:00:42", "home", "dog_walk_3", 87),
    Event("20:28:03", "patterson", "drinks", 79),
    Event("21:47:29", "patterson", "quiz", 43),
    Event("22:30:15", "home", "reading", 68),
    Event("23:38:21", "home", "sleep", 0),
]


class AudioDataGenerator:
    def __init__(self):
        self.device_id = str(uuid.uuid4())
        self.base_date = datetime(2025, 7, 1, 0, 0, 0, tzinfo=ZoneInfo('America/Chicago'))
        self.chunks = []
        
        # Initialize schedule with times
        for event in SCHEDULE:
            time_parts = event.time_str.split(':')
            event.start_time = self.base_date.replace(
                hour=int(time_parts[0]),
                minute=int(time_parts[1]),
                second=int(time_parts[2])
            )
    
    def create_minimal_wav_header(self, num_samples: int, sample_rate: int) -> bytes:
        """Create a minimal WAV header for our tiny audio clips."""
        # WAV header for 8-bit mono audio
        data_size = num_samples
        file_size = data_size + 36  # 44 byte header - 8
        
        header = b'RIFF'
        header += struct.pack('<I', file_size)
        header += b'WAVE'
        header += b'fmt '
        header += struct.pack('<I', 16)  # fmt chunk size
        header += struct.pack('<H', 1)   # PCM format
        header += struct.pack('<H', 1)   # Mono
        header += struct.pack('<I', sample_rate)
        header += struct.pack('<I', sample_rate)  # Byte rate
        header += struct.pack('<H', 1)   # Block align
        header += struct.pack('<H', 8)   # 8 bits per sample
        header += b'data'
        header += struct.pack('<I', data_size)
        
        return header
    
    def generate_minimal_audio(self, target_db: float, duration_sec: float = 0.1, 
                              sample_rate: int = 500) -> str:
        """Generate tiny audio clip at target dB level."""
        samples = int(sample_rate * duration_sec)  # 50 samples at 0.1 sec
        
        # Convert dB to amplitude (0 dB = 1.0, -60 dB = 0.001)
        # Using a scaling factor to get reasonable 8-bit values
        if target_db <= -60:
            amplitude = 1  # Nearly silent
        else:
            amplitude = 10 ** (target_db / 20) * 40  # Scale for 8-bit range
        
        # Generate white noise
        audio = np.random.normal(0, amplitude, samples)
        
        # Convert to 8-bit unsigned integers (0-255 range for WAV)
        audio_8bit = np.clip(audio + 128, 0, 255).astype(np.uint8)
        
        # Create minimal WAV format
        wav_header = self.create_minimal_wav_header(samples, sample_rate)
        wav_data = wav_header + audio_8bit.tobytes()
        
        # Base64 encode
        return base64.b64encode(wav_data).decode('ascii')
    
    def get_current_activity(self, timestamp: datetime) -> str:
        """Find which activity is happening at the given timestamp."""
        for i, event in enumerate(SCHEDULE):
            if i == len(SCHEDULE) - 1:
                return event.activity
            
            next_event = SCHEDULE[i + 1]
            if timestamp >= event.start_time and timestamp < next_event.start_time:
                return event.activity
        
        return "sleep"  # Default to sleep if outside schedule
    
    def generate_day_data(self) -> Dict:
        """Generate a full day of audio data."""
        # Start from wake time, skip the overnight sleep period
        current_time = SCHEDULE[0].start_time  # 07:23:14
        end_time = SCHEDULE[-1].start_time     # 23:38:21
        
        current_db = -45  # Start quiet at wake
        chunk_count = 0
        
        while current_time < end_time:
            # Find current activity
            activity = self.get_current_activity(current_time)
            target_db = ACTIVITY_DB_LEVELS.get(activity, -35)
            
            # Smooth transition to target dB (exponential smoothing)
            current_db = current_db * 0.7 + target_db * 0.3
            
            # Add natural variation
            chunk_db = current_db + random.uniform(-3, 3)
            chunk_db = max(-55, min(-15, chunk_db))  # Clamp to reasonable range
            
            # Generate audio data
            audio_data = self.generate_minimal_audio(chunk_db)
            
            # Create chunk metadata
            chunk_id = f"{chunk_count:04d}-{uuid.uuid4().hex[:8]}"
            timestamp_start = current_time.astimezone(timezone.utc)
            timestamp_end = timestamp_start + timedelta(seconds=30)
            
            chunk = {
                "id": chunk_id,
                "audio_data": audio_data,
                "timestamp_start": timestamp_start.strftime("%Y-%m-%dT%H:%M:%SZ"),
                "timestamp_end": timestamp_end.strftime("%Y-%m-%dT%H:%M:%SZ"),
                "duration": 30000,  # Claim 30 seconds in milliseconds
                "overlap_duration": 2,
                "audio_format": "wav",
                "sample_rate": 500  # Actual sample rate of our mini audio
            }
            
            self.chunks.append(chunk)
            chunk_count += 1
            
            # Move to next chunk (every 60 seconds for test data efficiency)
            current_time += timedelta(seconds=60)
        
        # Create the final payload structure matching iOS format
        return {
            "device_id": self.device_id,
            "batch_metadata": {
                "app_version": "1.0",
                "total_records": len(self.chunks)
            },
            "stream_name": "ios_mic",
            "data": self.chunks,
            "pipeline_activity_id": str(uuid.uuid4()),
            "timestamp": datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%S.%fZ")[:-3] + "Z"
        }


def main():
    """Generate and save the audio test data."""
    generator = AudioDataGenerator()
    
    print("Generating minimal iOS audio test data...")
    data = generator.generate_day_data()
    
    # Save uncompressed JSON for debugging
    output_file = 'test_data_ios_audio.json'
    with open(output_file, 'w') as f:
        json.dump(data, f, indent=2)
    
    # Save compressed version for actual use
    compressed_file = 'test_data_ios_audio.json.gz'
    with gzip.open(compressed_file, 'wt', encoding='utf-8') as f:
        json.dump(data, f, indent=2)
    
    # Calculate and display statistics
    num_chunks = len(data['data'])
    
    # Estimate file size
    avg_audio_size = len(data['data'][0]['audio_data']) if data['data'] else 0
    total_audio_size = avg_audio_size * num_chunks
    json_overhead = num_chunks * 300  # Estimate JSON metadata per chunk
    total_size_kb = (total_audio_size + json_overhead) / 1024
    
    # Get actual file sizes
    import os
    uncompressed_size = os.path.getsize(output_file) / 1024
    compressed_size = os.path.getsize(compressed_file) / 1024
    
    print(f"Generated {num_chunks} audio chunks")
    print(f"Device ID: {data['device_id']}")
    print(f"Time range: {data['data'][0]['timestamp_start']} to {data['data'][-1]['timestamp_end']}")
    print(f"Audio size per chunk: {avg_audio_size} bytes")
    print(f"Chunk interval: 60 seconds")
    print(f"\nFile sizes:")
    print(f"  Uncompressed: {uncompressed_size:.1f} KB ({output_file})")
    print(f"  Compressed:   {compressed_size:.1f} KB ({compressed_file})")
    print(f"  Compression ratio: {uncompressed_size/compressed_size:.1f}x")
    
    # Show sample of activities covered
    print("\nSample activities and chunks:")
    for i in range(0, min(10, num_chunks), max(1, num_chunks // 10)):
        chunk = data['data'][i]
        timestamp = datetime.fromisoformat(chunk['timestamp_start'].replace('Z', '+00:00'))
        timestamp_cdt = timestamp.astimezone(ZoneInfo('America/Chicago'))
        activity = generator.get_current_activity(timestamp_cdt.replace(tzinfo=ZoneInfo('America/Chicago')))
        print(f"  {timestamp_cdt.strftime('%H:%M:%S')} - {activity}")


if __name__ == "__main__":
    main()