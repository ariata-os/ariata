#!/usr/bin/env python3
"""
Script to update all test data dates to July 1, 2025
"""

import json
import re
from pathlib import Path
from datetime import datetime, timedelta

# Define date mappings
DATE_MAPPINGS = {
    # Main test day
    "2025-05-03": "2025-07-01",
    
    # Days around the main test day
    "2025-05-02": "2025-06-30",
    "2025-05-01": "2025-06-29",
    
    # January dates (for created/updated fields)
    "2025-01-01": "2025-06-01",
    "2025-01-15": "2025-06-15",
    
    # Other dates
    "2025-02-01": "2025-06-01",
    "2025-04-28": "2025-06-28",
    "2025-04-30": "2025-06-30",
}

def update_json_file(filepath):
    """Update dates in a JSON file"""
    print(f"Processing {filepath}...")
    
    with open(filepath, 'r') as f:
        content = f.read()
    
    original_content = content
    replacements = 0
    
    # Replace dates in the content
    for old_date, new_date in DATE_MAPPINGS.items():
        # Count replacements for this date
        count = len(re.findall(re.escape(old_date), content))
        if count > 0:
            content = content.replace(old_date, new_date)
            replacements += count
            print(f"  Replaced {count} instances of {old_date} with {new_date}")
    
    if replacements > 0:
        # Write the updated content back
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"  ✅ Total replacements: {replacements}")
    else:
        print(f"  ℹ️  No dates to replace")
    
    return replacements

def update_markdown_file(filepath):
    """Update dates in the TEST_DAY.md file"""
    print(f"Processing {filepath}...")
    
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Replace the main date reference
    content = content.replace("Date: May 3, 2025", "Date: July 1, 2025")
    content = content.replace("Saturday", "Tuesday")
    
    # Update any inline May 3 references
    content = content.replace("May 3, 2025", "July 1, 2025")
    
    # Write back
    with open(filepath, 'w') as f:
        f.write(content)
    
    print(f"  ✅ Updated markdown file")

def main():
    # Get the tests directory
    tests_dir = Path(__file__).parent.parent / "tests"
    
    if not tests_dir.exists():
        print(f"❌ Tests directory not found: {tests_dir}")
        return
    
    print(f"📅 Updating all test dates to July 1, 2025")
    print(f"📁 Working in: {tests_dir}\n")
    
    total_replacements = 0
    
    # Process JSON files
    json_files = [
        tests_dir / "test_data_google_calendar.json",
        tests_dir / "test_data_ios_location.json"
    ]
    
    for json_file in json_files:
        if json_file.exists():
            replacements = update_json_file(json_file)
            total_replacements += replacements
        else:
            print(f"⚠️  File not found: {json_file}")
    
    # Process markdown file
    md_file = tests_dir / "TEST_DAY.md"
    if md_file.exists():
        update_markdown_file(md_file)
    else:
        print(f"⚠️  File not found: {md_file}")
    
    print(f"\n✅ Complete! Updated {total_replacements} date references in JSON files")
    print("📅 All test data now uses July 1, 2025 as the reference date")

if __name__ == "__main__":
    main()