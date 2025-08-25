"""Celery tasks for syncing data sources - generic implementation."""

import asyncio
import os
import json
import logging
from datetime import datetime, timedelta
from typing import Dict, Any, Optional
import traceback
import importlib
from uuid import uuid4

from celery import Task
from sqlalchemy import text, select, update
from sqlalchemy.orm import sessionmaker

from sources.base.scheduler.celery_app import app
from sources.base.storage.database import sync_engine, SyncSessionLocal as Session
from .token_refresh import create_token_refresher


def json_serializable(obj):
    """Convert objects to JSON-serializable format."""
    if isinstance(obj, datetime):
        return obj.isoformat()
    elif isinstance(obj, dict):
        return {k: json_serializable(v) for k, v in obj.items()}
    elif isinstance(obj, list):
        return [json_serializable(v) for v in obj]
    else:
        return obj


# Set up logger
logger = logging.getLogger(__name__)


class SourceRegistry:
    """Registry for dynamically loading source sync classes."""
    
    @staticmethod
    def get_sync_class(stream_name: str, source_name: str):
        """Get sync class based on naming convention."""
        
        import yaml
        from pathlib import Path
        
        # Parse stream name to get the part without source prefix
        # e.g., 'google_calendar' -> 'calendar'
        stream_parts = stream_name.split('_', 1)
        if len(stream_parts) > 1 and stream_parts[0] == source_name:
            stream_suffix = stream_parts[1]
        else:
            stream_suffix = stream_name
        
        # Load stream configuration from _stream.yaml
        stream_yaml_path = Path(f'/sources/{source_name}/{stream_suffix}/_stream.yaml')
        if not stream_yaml_path.exists():
            raise ValueError(f"Stream configuration not found: {stream_yaml_path}")
        
        with open(stream_yaml_path) as f:
            stream_config = yaml.safe_load(f)
        
        # Check if this is a push stream (no sync needed)
        ingestion_type = stream_config.get('ingestion', {}).get('type')
        
        if ingestion_type == 'push':
            raise ValueError(f"Stream '{stream_name}' is push-only (no sync needed)")
        
        # Derive sync module and class based on naming convention
        # e.g., google_calendar -> sources.google.calendar.sync.GoogleCalendarSync
        # We already have stream_suffix from above
        stream_subdir = stream_suffix
        
        # Build module path
        sync_module = f"sources.{source_name}.{stream_subdir}.sync"
        
        # Derive sync class name from stream name
        # e.g., google_calendar -> GoogleCalendarSync
        sync_class = ''.join(p.capitalize() for p in stream_name.split('_')) + 'Sync'
        
        # Import and return the class
        try:
            module = importlib.import_module(sync_module)
            return getattr(module, sync_class)
        except ImportError as e:
            raise ValueError(f"Failed to import {sync_module}: {e}")
        except AttributeError as e:
            raise ValueError(f"Class {sync_class} not found in {sync_module}: {e}")


@app.task(name="sync_stream", bind=True,
          autoretry_for=(Exception,), retry_kwargs={'max_retries': 3})
def sync_stream(self, stream_id: str, manual: bool = False):
    """Generic task to sync any data stream."""

    # Use sync database session
    db = Session()
    try:
        # Get stream details with instance configuration
        result = db.execute(
            text("""
                SELECT stc.*, src.company, src.platform, src.auth_type,
                       s.id as stream_instance_id, s.initial_sync_type, s.initial_sync_days, s.initial_sync_days_future,
                       s.sync_schedule as instance_sync_schedule, s.settings as instance_settings, s.sync_cursor,
                       stc.last_ingestion_at
                FROM streams s
                JOIN stream_configs stc ON s.stream_config_id = stc.id
                JOIN source_configs src ON stc.source_name = src.name
                WHERE s.id = :stream_id
            """),
            {"stream_id": stream_id}
        ).first()

        if not result:
            raise ValueError(f"Stream {stream_id} not found")

        stream = dict(result._mapping)
        source_name = stream['source_name']
        stream_name = stream['stream_name']
        platform = stream.get('platform', 'cloud')

        # Skip device sources - they push their own data
        if platform == 'device' and not manual:
            return {"status": "skipped", "reason": "device_source"}

        # Skip inactive streams unless manually triggered
        if stream['status'] != 'active' and not manual:
            return {"status": "skipped", "reason": "stream_inactive"}

        # Get the sync class for this stream (not source)
        try:
            sync_class = SourceRegistry.get_sync_class(stream_name, source_name)
        except ValueError as e:
            # Source doesn't have a sync implementation (might be webhook-only)
            return {"status": "skipped", "reason": str(e)}

        # Create ingestion run
        ingestion_run_id = uuid4()

        db.execute(
            text("""
                INSERT INTO pipeline_activities 
                (id, activity_type, activity_name, source_name, stream_id, 
                 status, started_at, created_at, updated_at) 
                VALUES (:id, :activity_type, :activity_name, :source_name, 
                        :stream_id, :status, :started_at, :created_at, :updated_at)
            """),
            {
                "id": str(ingestion_run_id),
                "activity_type": "ingestion",
                "activity_name": f"{source_name}_stream_ingestion",
                "source_name": source_name,
                "stream_id": stream.get('stream_instance_id'),  # Use the stream instance ID, not config ID
                "status": "running",
                "started_at": datetime.utcnow(),
                "created_at": datetime.utcnow(),
                "updated_at": datetime.utcnow()
            }
        )
        db.commit()

        try:
            # Check if source requires credentials
            oauth_credentials = None
            if stream.get('auth_type') == 'oauth2':
                # Get OAuth tokens from the sources table
                source_result = db.execute(
                    text("""
                        SELECT id, oauth_access_token, oauth_refresh_token, oauth_expires_at, scopes
                        FROM sources
                        WHERE source_name = :source_name
                        AND oauth_access_token IS NOT NULL
                        AND status = 'active'
                        LIMIT 1
                    """),
                    {
                        "source_name": source_name
                    }
                ).first()

                if source_result:
                    oauth_credentials = dict(source_result._mapping)
                    oauth_credentials['source_id'] = oauth_credentials['id']
                else:
                    raise ValueError(
                        f"No authenticated source found for {source_name}")

            # Create stream object with necessary fields
            stream_obj = StreamWrapper(stream, oauth_credentials)

            # Initialize sync class
            if oauth_credentials and hasattr(sync_class, '__init__') and oauth_credentials.get('oauth_access_token'):
                # For OAuth sources that need tokens
                sync = sync_class(
                    stream_obj,
                    oauth_credentials['oauth_access_token'],
                    token_refresher=create_token_refresher(
                        source_name, oauth_credentials, stream, db) if oauth_credentials.get('oauth_refresh_token') else None
                )
            else:
                # For sources that don't need tokens (e.g., device-based)
                sync = sync_class(stream_obj)

            # Run sync - handle both async and sync implementations
            if asyncio.iscoroutinefunction(sync.run):
                # Async sync implementation
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                try:
                    stats = loop.run_until_complete(sync.run())
                finally:
                    loop.close()
            else:
                # Sync implementation
                stats = sync.run()

            # Update stream last_ingestion_at
            db.execute(
                text("""
                    UPDATE stream_configs 
                    SET last_ingestion_at = :last_ingestion_at,
                        updated_at = :updated_at
                    WHERE id = :id
                """),
                {
                    "id": stream['id'],  # Use the stream_config id from the query result
                    "last_ingestion_at": datetime.utcnow(),
                    "updated_at": datetime.utcnow()
                }
            )
            
            # Update sync cursor if provided and stream instance exists
            if stats.get("next_sync_token") and stream.get("stream_instance_id"):
                db.execute(
                    text("""
                        UPDATE streams 
                        SET sync_cursor = :sync_cursor,
                            last_sync_at = :last_sync_at,
                            last_sync_status = 'success',
                            updated_at = :updated_at
                        WHERE id = :id
                    """),
                    {
                        "id": stream["stream_instance_id"],
                        "sync_cursor": stats["next_sync_token"],
                        "last_sync_at": datetime.utcnow(),
                        "updated_at": datetime.utcnow()
                    }
                )

            # Update ingestion run
            db.execute(
                text("""
                    UPDATE pipeline_activities 
                    SET status = :status,
                        completed_at = :completed_at,
                        records_processed = :records_processed,
                        updated_at = :updated_at,
                        activity_metadata = :activity_metadata
                    WHERE id = :id
                """),
                {
                    "id": str(ingestion_run_id),
                    "status": "completed",
                    "completed_at": datetime.utcnow(),
                    "records_processed": stats.get("records_processed", stats.get("events_processed", stats.get("locations_processed", 0))),
                    "updated_at": datetime.utcnow(),
                    "activity_metadata": json.dumps(json_serializable(stats))
                }
            )

            db.commit()

            return {
                "status": "success",
                "stream_id": stream_id,
                "stream_name": stream['stream_name'],
                "source": source_name,
                "stats": stats
            }

        except Exception as e:
            # Log error
            error_message = f"{type(e).__name__}: {str(e)}"
            traceback.print_exc()

            # Classify errors - don't retry certain types
            non_retryable_errors = (
                'ProgrammingError',  # Database schema issues
                'UndefinedColumn',   # Missing columns
                'AuthenticationError', # OAuth/auth issues that need user intervention
                'PermissionError',   # Access denied
                'Unauthorized'       # 401 errors
            )
            
            should_retry = not any(error_type in error_message for error_type in non_retryable_errors)

            # Update pipeline activity
            db.execute(
                text("""
                    UPDATE pipeline_activities 
                    SET status = :status,
                        completed_at = :completed_at,
                        error_message = :error_message,
                        updated_at = :updated_at
                    WHERE id = :id
                """),
                {
                    "id": str(ingestion_run_id),
                    "status": "failed",
                    "completed_at": datetime.utcnow(),
                    # Truncate if too long
                    "error_message": error_message[:1000],
                    "updated_at": datetime.utcnow()
                }
            )

            db.commit()
            
            # Custom retry logic with exponential backoff
            if should_retry and self.request.retries < self.max_retries:
                # Exponential backoff: 60s, 120s, 240s
                countdown = 60 * (2 ** self.request.retries)
                raise self.retry(countdown=countdown, exc=e)
            else:
                # Don't retry or max retries reached
                raise

    finally:
        db.close()


# Keep the old sync_source task for backward compatibility
@app.task(name="sync_source", bind=True,
          autoretry_for=(Exception,), retry_kwargs={'max_retries': 3})
def sync_source(self, signal_id: str, manual: bool = False):
    """Generic task to sync any data source."""

    # Use sync database session
    db = Session()
    try:
        # Get signal details
        result = db.execute(
            text("""
                SELECT s.*, src.company, src.platform
                FROM signals s 
                JOIN sources src ON s.source_name = src.name
                WHERE s.id = :signal_id
            """),
            {"signal_id": signal_id}
        ).first()

        if not result:
            raise ValueError(f"Signal {signal_id} not found")

        signal = dict(result._mapping)
        source_name = signal['source_name']
        platform = signal.get('platform', 'cloud')

        # Skip device sources - they push their own data
        if platform == 'device' and not manual:
            return {"status": "skipped", "reason": "device_source"}

        # Skip inactive signals unless manually triggered
        if signal['status'] != 'active' and not manual:
            return {"status": "skipped", "reason": "signal_inactive"}

        # Get the sync class for this source
        try:
            sync_class = SourceRegistry.get_sync_class(source_name)
        except ValueError as e:
            # Source doesn't have a sync implementation (might be webhook-only)
            return {"status": "skipped", "reason": str(e)}

        # Create ingestion run
        ingestion_run_id = uuid4()

        # Get source_name from signal
        signal_info = db.execute(
            text("SELECT source_name FROM signals WHERE id = :signal_id"),
            {"signal_id": signal_id}
        ).first()

        if not signal_info:
            raise ValueError(f"Signal {signal_id} not found")

        db.execute(
            text("""
                INSERT INTO pipeline_activities 
                (id, activity_type, activity_name, source_name, signal_id, 
                 status, started_at, created_at, updated_at) 
                VALUES (:id, :activity_type, :activity_name, :source_name, 
                        :signal_id, :status, :started_at, :created_at, :updated_at)
            """),
            {
                "id": str(ingestion_run_id),
                "activity_type": "ingestion",
                "activity_name": f"{signal_info.source_name}_ingestion",
                "source_name": signal_info.source_name,
                "signal_id": signal_id,
                "status": "running",
                "started_at": datetime.utcnow(),
                "created_at": datetime.utcnow(),
                "updated_at": datetime.utcnow()
            }
        )
        db.commit()

        try:
            # Check if source requires credentials
            oauth_credentials = None
            if hasattr(sync_class, 'requires_credentials') and sync_class.requires_credentials:
                # Get OAuth tokens from the sources table
                source_result = db.execute(
                    text("""
                        SELECT oauth_access_token, oauth_refresh_token, oauth_expires_at, scopes
                        FROM sources
                        WHERE source_name = :source_name
                        AND oauth_access_token IS NOT NULL
                        LIMIT 1
                    """),
                    {
                        "source_name": source_name
                    }
                ).first()

                if source_result:
                    oauth_credentials = dict(source_result._mapping)

            # Create signal object with necessary fields
            signal_obj = SignalWrapper(signal, oauth_credentials)

            # Initialize sync class
            if oauth_credentials and hasattr(sync_class, '__init__') and oauth_credentials.get('oauth_access_token'):
                # For OAuth sources that need tokens
                sync = sync_class(
                    signal_obj,
                    oauth_credentials['oauth_access_token'],
                    token_refresher=create_token_refresher(
                        source_name, oauth_credentials, signal_obj, db) if oauth_credentials.get('oauth_refresh_token') else None
                )
            else:
                # For sources that don't need tokens (e.g., device-based)
                sync = sync_class(signal_obj)

            # Run sync - handle both async and sync implementations
            if asyncio.iscoroutinefunction(sync.run):
                # Async sync implementation
                loop = asyncio.new_event_loop()
                asyncio.set_event_loop(loop)
                try:
                    stats = loop.run_until_complete(sync.run())
                finally:
                    loop.close()
            else:
                # Sync implementation
                stats = sync.run()

            # Update signal
            update_data = {"last_successful_ingestion_at": datetime.utcnow()}
            if stats.get("next_sync_token"):
                update_data["sync_token"] = stats["next_sync_token"]

            db.execute(
                text("""
                    UPDATE signals 
                    SET last_successful_ingestion_at = :last_successful_ingestion_at,
                        sync_token = :sync_token,
                        updated_at = :updated_at
                    WHERE id = :id
                """),
                {
                    "id": signal_id,
                    "last_successful_ingestion_at": update_data["last_successful_ingestion_at"],
                    "sync_token": update_data.get("sync_token", signal.get('sync_token')),
                    "updated_at": datetime.utcnow()
                }
            )

            # Update ingestion run
            db.execute(
                text("""
                    UPDATE pipeline_activities 
                    SET status = :status,
                        completed_at = :completed_at,
                        records_processed = :records_processed,
                        updated_at = :updated_at,
                        activity_metadata = :activity_metadata
                    WHERE id = :id
                """),
                {
                    "id": str(ingestion_run_id),
                    "status": "completed",
                    "completed_at": datetime.utcnow(),
                    "records_processed": stats.get("records_processed", stats.get("events_processed", stats.get("locations_processed", 0))),
                    "updated_at": datetime.utcnow(),
                    "activity_metadata": json.dumps(json_serializable(stats))
                }
            )

            db.commit()

            return {
                "status": "success",
                "signal_id": signal_id,
                "source": source_name,
                "stats": stats
            }

        except Exception as e:
            # Log error
            error_message = f"{type(e).__name__}: {str(e)}"
            traceback.print_exc()

            # Update pipeline activity
            db.execute(
                text("""
                    UPDATE pipeline_activities 
                    SET status = :status,
                        completed_at = :completed_at,
                        error_message = :error_message,
                        updated_at = :updated_at
                    WHERE id = :id
                """),
                {
                    "id": str(ingestion_run_id),
                    "status": "failed",
                    "completed_at": datetime.utcnow(),
                    # Truncate if too long
                    "error_message": error_message[:1000],
                    "updated_at": datetime.utcnow()
                }
            )

            db.commit()
            raise

    finally:
        db.close()


class SignalWrapper:
    """Wrapper to make dict behave like an object for compatibility."""

    def __init__(self, signal_dict: dict, oauth_credentials: Optional[dict] = None):
        self._dict = signal_dict
        self._oauth_credentials = oauth_credentials

    def __getattr__(self, name):
        return self._dict.get(name)

    def __getitem__(self, key):
        return self._dict[key]

    @property
    def id(self):
        return self._dict['id']

    @property
    def signal_id(self):
        return self._dict['signal_id']

    @property
    def source_name(self):
        return self._dict['source_name']

    @property
    def sync_token(self):
        return self._dict.get('sync_token')

    @property
    def last_successful_ingestion_at(self):
        return self._dict.get('last_successful_ingestion_at')

    @property
    def is_active(self):
        return self._dict.get('status') == 'active'

    @property
    def fidelity_score(self):
        return self._dict.get('fidelity_score', 0.5)

    @property
    def description(self):
        return self._dict.get('description')

    @property
    def settings(self):
        return self._dict.get('settings', {})

    @property
    def device_token(self):
        return self._dict.get('device_token')

    @property
    def device_id_fk(self):
        return self._dict.get('device_id_fk')

    @property
    def signal_type(self):
        return self._dict.get('signal_type')

    @property
    def unit(self):
        return self._dict.get('unit_ucum')

    @property
    def computation(self):
        return self._dict.get('computation')


class StreamWrapper:
    """Wrapper to make stream dict behave like a signal object for compatibility."""

    def __init__(self, stream_dict: dict, oauth_credentials: Optional[dict] = None):
        self._dict = stream_dict
        self._oauth_credentials = oauth_credentials

    def __getattr__(self, name):
        return self._dict.get(name)

    def __getitem__(self, key):
        return self._dict[key]

    @property
    def id(self):
        return self._dict['id']

    @property
    def source_name(self):
        return self._dict['source_name']

    @property
    def stream_name(self):
        return self._dict['stream_name']

    @property
    def last_successful_ingestion_at(self):
        # For streams, we use last_ingestion_at
        return self._dict.get('last_ingestion_at')
    
    @property
    def sync_cursor(self):
        # Return the sync cursor for incremental syncs
        return self._dict.get('sync_cursor')
    
    @property
    def sync_token(self):
        # Alias for compatibility with Google Calendar
        return self.sync_cursor

    @property
    def is_active(self):
        return self._dict.get('status') == 'active'

    @property
    def settings(self):
        # Merge instance settings with config settings
        config_settings = self._dict.get('settings', {})
        instance_settings = self._dict.get('instance_settings', {})
        return {**config_settings, **instance_settings}
    
    @property
    def initial_sync_type(self):
        # Use instance value if available, otherwise fall back to default
        return self._dict.get('initial_sync_type', 'limited')
    
    @property
    def initial_sync_days(self):
        # Use instance value if available, otherwise fall back to default
        return self._dict.get('initial_sync_days', 90)
    
    @property
    def initial_sync_days_future(self):
        # Use instance value if available, otherwise fall back to default
        return self._dict.get('initial_sync_days_future', 30)
    
    @property
    def sync_schedule(self):
        # Use instance schedule if available, otherwise config schedule
        return self._dict.get('instance_sync_schedule') or self._dict.get('sync_schedule')






