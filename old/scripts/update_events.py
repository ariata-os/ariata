#!/usr/bin/env python3
"""Quick script to update all events in generate_w5h_events.py"""

import re

# Read the file
with open('generate_w5h_events.py', 'r') as f:
    content = f.read()

# Pattern to find W5HEvent creations with old fields
pattern = r'W5HEvent\((.*?)(?:category=.*?|mood=.*?|energy_level=.*?|productivity_score=.*?)\)'

# Function to clean up event creation
def clean_event(match):
    event_str = match.group(0)

    # Remove the old fields
    event_str = re.sub(r',\s*category=[^,\)]*', '', event_str)
    event_str = re.sub(r',\s*mood=[^,\)]*', '', event_str)
    event_str = re.sub(r',\s*energy_level=[^,\)]*', '', event_str)
    event_str = re.sub(r',\s*productivity_score=[^,\)]*', '', event_str)

    return event_str

# Apply the replacement
updated = re.sub(pattern, clean_event, content, flags=re.DOTALL)

# Save back
with open('generate_w5h_events.py', 'w') as f:
    f.write(updated)

print("Updated events to remove old fields")