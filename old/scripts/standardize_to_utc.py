#!/usr/bin/env python3
"""
Script to standardize all test data timestamps to UTC format
Converts Central Time (-05:00) timestamps to UTC by adding 5 hours
"""

import json
import re
from pathlib import Path
from datetime import datetime, timedelta, timezone

def convert_central_to_utc(datetime_str):
    """Convert a Central Time datetime string to UTC"""
    # Parse the datetime with timezone
    # Format: 2025-07-01T10:15:00-05:00
    if "-05:00" in datetime_str:
        # Remove the timezone suffix and parse
        base_time = datetime_str.replace("-05:00", "")
        dt = datetime.fromisoformat(base_time)
        # Add 5 hours to convert from CT to UTC
        utc_dt = dt + timedelta(hours=5)
        # Format as UTC with Z suffix
        return utc_dt.strftime("%Y-%m-%dT%H:%M:%SZ")
    return datetime_str

def update_google_calendar_to_utc(filepath):
    """Update Google Calendar data to use UTC timestamps"""
    print(f"Processing {filepath}...")
    
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Count original occurrences
    original_count = content.count('-05:00')
    
    if original_count > 0:
        # Use regex to find and convert all datetime strings with -05:00
        def replace_datetime(match):
            datetime_str = match.group(1)  # Get the captured group without quotes
            # Parse and convert to UTC
            dt = datetime.fromisoformat(datetime_str)
            # Add 5 hours to convert from CT to UTC
            utc_dt = dt + timedelta(hours=5)
            # Format as UTC with Z suffix
            return f'"{utc_dt.strftime("%Y-%m-%dT%H:%M:%SZ")}"'
        
        # Pattern to match datetime strings with -05:00
        pattern = r'"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})-05:00"'
        content = re.sub(pattern, replace_datetime, content)
        
        # Remove timezone fields since we're using UTC
        content = re.sub(r',?\s*"timeZone":\s*"[^"]*"', '', content)
        
        # Write the updated content back
        with open(filepath, 'w') as f:
            f.write(content)
        
        print(f"  ✅ Converted {original_count} Central Time timestamps to UTC")
    else:
        print(f"  ℹ️  No Central Time timestamps to convert")
    
    return original_count

def verify_ios_location_utc(filepath):
    """Verify iOS location data is already in UTC"""
    print(f"Verifying {filepath}...")
    
    with open(filepath, 'r') as f:
        data = json.load(f)
    
    # Check a sample of timestamps
    sample_count = min(5, len(data.get('locations', [])))
    for i in range(sample_count):
        location = data['locations'][i]
        timestamp = location.get('timestamp', '')
        if timestamp.endswith('Z'):
            print(f"  ✓ Already UTC: {timestamp}")
        else:
            print(f"  ⚠️  Not UTC format: {timestamp}")
    
    return True

def update_test_day_to_utc(filepath):
    """Update TEST_DAY.md to note that all times are in UTC"""
    print(f"Processing {filepath}...")
    
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Add UTC note if not already present
    if "UTC" not in content and "July 1, 2025" in content:
        # Add note after the date line
        content = content.replace(
            "Date: July 1, 2025",
            "Date: July 1, 2025 (All times in UTC)"
        )
        
        # Write back
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"  ✅ Added UTC notation to markdown")
    else:
        print(f"  ℹ️  UTC notation already present or not needed")

def main():
    # Get the tests directory
    tests_dir = Path(__file__).parent.parent / "tests"
    
    if not tests_dir.exists():
        print(f"❌ Tests directory not found: {tests_dir}")
        return
    
    print(f"🌐 Standardizing all test data to UTC format")
    print(f"📁 Working in: {tests_dir}\n")
    
    total_conversions = 0
    
    # Process Google Calendar file (needs conversion from Central Time)
    google_cal_file = tests_dir / "test_data_google_calendar.json"
    if google_cal_file.exists():
        conversions = update_google_calendar_to_utc(google_cal_file)
        total_conversions += conversions
    else:
        print(f"⚠️  File not found: {google_cal_file}")
    
    # Verify iOS location file (should already be UTC)
    ios_location_file = tests_dir / "test_data_ios_location.json"
    if ios_location_file.exists():
        verify_ios_location_utc(ios_location_file)
    else:
        print(f"⚠️  File not found: {ios_location_file}")
    
    # Update markdown file
    md_file = tests_dir / "TEST_DAY.md"
    if md_file.exists():
        update_test_day_to_utc(md_file)
    else:
        print(f"⚠️  File not found: {md_file}")
    
    print(f"\n✅ Complete! Converted {total_conversions} timestamps to UTC")
    print("🌐 All test data now uses UTC timestamps")

if __name__ == "__main__":
    main()