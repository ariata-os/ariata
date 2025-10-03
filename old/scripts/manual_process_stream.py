#!/usr/bin/env python3
"""Manually trigger processing of existing stream data in MinIO."""

import sys
import os
sys.path.insert(0, '/app')
os.chdir('/app')

from sources.base.scheduler.celery_app import app
from uuid import uuid4

# Stream details
stream_name = 'google_calendar'
stream_key = 'streams/google_calendar_events/2025/08/10/27ba3a4728564525b6fdfa0be2e66074.json'
stream_id = '31f2ac84-632b-491c-ba38-faa429f7d0b6'

# Queue the processing task
task = app.send_task(
    'process_stream_batch',
    args=[
        stream_name,       # stream_name
        stream_key,        # MinIO key
        str(uuid4()),      # new pipeline_activity_id
        stream_id          # stream_id
    ]
)

print(f'Successfully queued processing task: {task.id}')
print(f'Stream: {stream_name}')
print(f'Key: {stream_key}')
print(f'Stream ID: {stream_id}')