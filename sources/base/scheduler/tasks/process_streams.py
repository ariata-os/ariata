"""Celery tasks for processing stream batches from MinIO and storing in PostgreSQL."""

import os
import json
import traceback
import importlib
import yaml
from pathlib import Path
from datetime import datetime, timedelta, timezone as tz
from typing import Dict, Any, Optional
from uuid import uuid4

import aioboto3
import asyncio
from sqlalchemy import text
from sqlalchemy.orm import sessionmaker
from botocore.config import Config

from sources.base.scheduler.celery_app import app
from sources.base.storage.minio import get_minio_config
from sources.base.storage.database import sync_engine, SyncSessionLocal as Session

# Get MinIO configuration
_minio_config = get_minio_config()
MINIO_ENDPOINT = _minio_config['endpoint']
MINIO_ACCESS_KEY = _minio_config['access_key']
MINIO_SECRET_KEY = _minio_config['secret_key']
MINIO_BUCKET = _minio_config['bucket']
MINIO_USE_SSL = _minio_config['use_ssl']

# Cache for stream configurations (loaded on demand)
STREAM_CONFIGS_CACHE = {}


def get_stream_config(source_name: str, stream_name: str) -> Dict[str, Any]:
    """Load stream configuration from _stream.yaml file.
    
    Args:
        source_name: The source name (e.g., 'google')
        stream_name: The stream name without source prefix (e.g., 'calendar')
        
    Returns:
        Stream configuration dictionary
    """
    cache_key = f"{source_name}_{stream_name}"
    
    # Check cache first
    if cache_key in STREAM_CONFIGS_CACHE:
        return STREAM_CONFIGS_CACHE[cache_key]
    
    # Load from YAML file
    stream_yaml_path = Path(f'/sources/{source_name}/{stream_name}/_stream.yaml')
    if not stream_yaml_path.exists():
        raise ValueError(f"Stream configuration not found: {stream_yaml_path}")
    
    with open(stream_yaml_path) as f:
        config = yaml.safe_load(f)
    
    # Cache for future use
    STREAM_CONFIGS_CACHE[cache_key] = config
    return config


async def get_stream_data(stream_key: str) -> Dict[str, Any]:
    """Retrieve stream data from MinIO."""
    session = aioboto3.Session()
    async with session.client(
        's3',
        endpoint_url=MINIO_ENDPOINT,  # Already includes protocol
        aws_access_key_id=MINIO_ACCESS_KEY,
        aws_secret_access_key=MINIO_SECRET_KEY,
        config=Config(signature_version='s3v4'),
        region_name='us-east-1'
    ) as s3:
        response = await s3.get_object(Bucket=MINIO_BUCKET, Key=stream_key)
        data = await response['Body'].read()
        return json.loads(data.decode('utf-8'))


def get_processor_class(source_name: str, stream_name: str):
    """Get processor class based on naming convention.
    
    Args:
        source_name: The source name (e.g., 'google')
        stream_name: The full stream name (e.g., 'google_calendar')
        
    Returns:
        Processor class
    """
    # Parse stream name to get the part without source prefix
    # e.g., 'google_calendar' -> 'calendar'
    stream_parts = stream_name.split('_', 1)
    if len(stream_parts) > 1 and stream_parts[0] == source_name:
        stream_suffix = stream_parts[1]
    else:
        stream_suffix = stream_name
    
    # Build processor module path
    # e.g., google/calendar -> sources.google.calendar.processor
    processor_module_path = f"sources.{source_name}.{stream_suffix}.processor"
    
    # Derive processor class name from stream name
    # e.g., google_calendar -> GoogleCalendarStreamProcessor
    class_parts = stream_name.split('_')
    processor_class_name = ''.join(p.capitalize() for p in class_parts) + 'StreamProcessor'
    
    print(f"[StreamProcessor] Loading processor: {processor_module_path}.{processor_class_name} for stream {stream_name}")
    
    try:
        # Import the processor module
        processor_module = importlib.import_module(processor_module_path)
        
        # Get the processor class directly by name
        if not hasattr(processor_module, processor_class_name):
            raise ValueError(f"Class {processor_class_name} not found in module {processor_module_path}")
        
        return getattr(processor_module, processor_class_name)
        
    except Exception as e:
        raise ValueError(f"Failed to load processor for {stream_name}: {e}")


def process_stream_generic(
    stream_data: Dict[str, Any],
    stream_name: str,
    stream_id: str,
    source_id: str,
    db
) -> Dict[str, Any]:
    """Process stream data using the new PostgreSQL storage pattern.
    
    Dynamically loads the appropriate processor and stores data directly in PostgreSQL.
    Large assets (audio, images) are stored in MinIO with references in PostgreSQL.
    """
    
    # Get stream and source info from database
    stream_info = db.execute(
        text("""
            SELECT sc.source_name, sc.stream_name
            FROM streams s
            JOIN stream_configs sc ON s.stream_config_id = sc.id
            WHERE s.id = :stream_id
        """),
        {"stream_id": stream_id}
    )
    stream_record = stream_info.fetchone()
    if not stream_record:
        raise ValueError(f"Stream not found: {stream_id}")
    
    source_name = stream_record.source_name
    db_stream_name = stream_record.stream_name
    
    try:
        # Get processor class using naming convention
        processor_class = get_processor_class(source_name, db_stream_name)
        
        # Instantiate the processor (config is auto-loaded in __init__)
        processor = processor_class()
        
        # Process the batch - run async function in sync context
        loop = asyncio.new_event_loop()
        try:
            result = loop.run_until_complete(processor.process_batch(stream_data, source_id))
        finally:
            loop.close()
        
        print(f"[StreamProcessor] Successfully processed {result.get('records_processed', 0)} records for {db_stream_name}")
        
        return {
            "status": "completed",
            "stream_name": db_stream_name,
            "source_name": source_name,
            "records_processed": result.get('records_processed', 0),
            "records_stored": result.get('records_stored', 0),
            "minio_assets": result.get('minio_assets', 0)
        }
        
    except Exception as e:
        print(f"[StreamProcessor] Error processing stream {db_stream_name}: {e}")
        raise
    

@app.task(name="process_stream_batch", bind=True,
          autoretry_for=(Exception,), retry_kwargs={'max_retries': 3})
def process_stream_batch(
    self, 
    stream_name: str, 
    stream_key: str, 
    pipeline_activity_id: str,
    stream_id: str
):
    """
    Process a stream batch from MinIO storage.
    
    Args:
        stream_name: Name of the stream (e.g., 'apple_ios_core_location')
        stream_key: MinIO key where the stream data is stored
        pipeline_activity_id: ID of the pipeline activity from ingestion
        stream_id: ID of the stream configuration
    """
    
    print(f"[DEBUG] process_stream_batch called with: stream_name={stream_name}, stream_key={stream_key}, stream_id={stream_id}")
    
    db = Session()
    signal_creation_id = None
    
    try:
        # Get source from database, including stream_config_id for FK
        stream_result = db.execute(
            text("""
                SELECT sc.id as stream_config_id, sc.source_name, sc.stream_name as db_stream_name
                FROM streams s
                JOIN stream_configs sc ON s.stream_config_id = sc.id
                WHERE s.id = :stream_id
            """),
            {"stream_id": stream_id}
        )
        stream_record = stream_result.fetchone()
        if not stream_record:
            raise ValueError(f"Stream not found: {stream_id}")
        
        stream_config_id = stream_record.stream_config_id
        source_name = stream_record.source_name
        actual_stream_name = stream_record.db_stream_name
        print(f"[DEBUG] Database lookup: stream_id={stream_id} -> stream_name={actual_stream_name}, source_name={source_name}, stream_config_id={stream_config_id}")
        
        # Create a new pipeline activity for signal creation
        signal_creation_id = str(uuid4())
        # Note: stream_id in pipeline_activities expects stream_configs.id, not streams.id
        # For now, we'll set it to NULL since it's optional
        db.execute(
            text("""
                INSERT INTO pipeline_activities 
                (id, activity_type, activity_name, source_name, stream_id,
                 status, started_at, created_at, updated_at) 
                VALUES (:id, :activity_type, :activity_name, :source_name, 
                        :stream_id, :status, :started_at, :created_at, :updated_at)
            """),
            {
                "id": signal_creation_id,
                "activity_type": "signal_creation",
                "activity_name": f"{stream_name}_signal_creation",
                "source_name": source_name,
                "stream_id": stream_id,  # Use stream_id (streams.id) for the FK
                "status": "running",
                "started_at": datetime.utcnow(),
                "created_at": datetime.utcnow(),
                "updated_at": datetime.utcnow()
            }
        )
        db.commit()
        
        # Get stream data from MinIO
        loop = asyncio.new_event_loop()
        try:
            stream_data = loop.run_until_complete(get_stream_data(stream_key))
        finally:
            loop.close()
        
        # Process using generic function
        print(f"[DEBUG] About to process with process_stream_generic: stream_name={stream_name}, stream_id={stream_id}")
        result = process_stream_generic(stream_data, stream_name, stream_id, db)
        
        # Update signal creation pipeline activity
        db.execute(
            text("""
                UPDATE pipeline_activities 
                SET status = :status,
                    completed_at = :completed_at,
                    output_path = :output_path,
                    updated_at = :updated_at,
                    records_processed = :records_processed
                WHERE id = :id
            """),
            {
                "id": signal_creation_id,
                "status": "completed",
                "completed_at": datetime.utcnow(),
                "output_path": stream_key,
                "updated_at": datetime.utcnow(),
                "records_processed": result.get("records_processed", 0)
            }
        )
        
        # Also update the stream's last successful processing time
        db.execute(
            text("""
                UPDATE stream_configs 
                SET last_ingestion_at = :last_ingestion_at,
                    updated_at = :updated_at
                WHERE id = :stream_id
            """),
            {
                "stream_id": stream_id,
                "last_ingestion_at": datetime.now(tz.utc),
                "updated_at": datetime.now(tz.utc)
            }
        )
        
        db.commit()
        
        # TRANSITION DETECTION DISABLED - Part of signal generation removal
        print(f"[REFACTOR] Transition detection disabled - no signals were created")
        
        return result
        
    except Exception as e:
        error_message = f"{type(e).__name__}: {str(e)}"
        traceback.print_exc()
        
        # Update signal creation pipeline activity with failure if we created one
        if signal_creation_id:
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
                    "id": signal_creation_id,
                    "status": "failed",
                    "completed_at": datetime.utcnow(),
                    "error_message": error_message,
                    "updated_at": datetime.utcnow()
                }
            )
            db.commit()
        raise


@app.task(name="process_stream_data", bind=True,
          autoretry_for=(Exception,), retry_kwargs={'max_retries': 3})
def process_stream_data(
    self,
    stream_name: str,
    data: Dict[str, Any],
    source_id: str,
    stream_id: str,
    pipeline_activity_id: str
):
    """
    Process stream data directly without MinIO storage.
    
    This new task receives data directly from the ingestion endpoint
    and processes it according to the stream type. Structured data goes
    directly to PostgreSQL, while binary data (audio, images) goes to MinIO.
    
    Args:
        stream_name: Name of the stream (e.g., 'ios_location', 'google_calendar')
        data: Raw data from ingestion endpoint
        source_id: ID of the source
        stream_id: ID of the stream instance
        pipeline_activity_id: ID of the pipeline activity from ingestion
    """
    
    print(f"[process_stream_data] Processing {stream_name} with {len(str(data))} bytes of data")
    
    db = Session()
    
    try:
        # Get source info from database
        source_result = db.execute(
            text("""
                SELECT source_name, instance_name
                FROM sources
                WHERE id = :source_id
            """),
            {"source_id": source_id}
        )
        source_record = source_result.fetchone()
        if not source_record:
            raise ValueError(f"Source not found: {source_id}")
        
        source_name = source_record.source_name
        
        # Get processor class using naming convention
        processor_class = get_processor_class(source_name, stream_name)
        
        # Instantiate the processor (config is auto-loaded in __init__)
        processor = processor_class()
        
        # Process the batch directly - run async function in sync context
        loop = asyncio.new_event_loop()
        try:
            result = loop.run_until_complete(processor.process_batch(data, source_id))
        finally:
            loop.close()
        
        print(f"[process_stream_data] Successfully processed {result.get('records_processed', 0)} records for {stream_name}")
        print(f"[process_stream_data] Stored {result.get('records_stored', 0)} to PostgreSQL, {result.get('minio_assets', 0)} to MinIO")
        
        # Update pipeline activity with success
        db.execute(
            text("""
                UPDATE pipeline_activities 
                SET status = :status,
                    completed_at = :completed_at,
                    records_processed = :records_processed,
                    activity_metadata = :metadata,
                    updated_at = :updated_at
                WHERE id = :id
            """),
            {
                "id": pipeline_activity_id,
                "status": "completed",
                "completed_at": datetime.utcnow(),
                "records_processed": result.get('records_processed', 0),
                "metadata": json.dumps({
                    "records_stored": result.get('records_stored', 0),
                    "minio_assets": result.get('minio_assets', 0)
                }),
                "updated_at": datetime.utcnow()
            }
        )
        
        # Update stream's last successful processing time
        db.execute(
            text("""
                UPDATE streams 
                SET last_sync_at = :last_sync_at,
                    updated_at = :updated_at
                WHERE id = :stream_id
            """),
            {
                "stream_id": stream_id,
                "last_sync_at": datetime.utcnow(),
                "updated_at": datetime.utcnow()
            }
        )
        
        db.commit()
        
        return {
            "status": "completed",
            "stream_name": stream_name,
            "source_name": source_name,
            "records_processed": result.get('records_processed', 0),
            "records_stored": result.get('records_stored', 0),
            "minio_assets": result.get('minio_assets', 0)
        }
        
    except Exception as e:
        error_message = f"{type(e).__name__}: {str(e)}"
        traceback.print_exc()
        
        # Update pipeline activity with failure
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
                "id": pipeline_activity_id,
                "status": "failed",
                "completed_at": datetime.utcnow(),
                "error_message": error_message,
                "updated_at": datetime.utcnow()
            }
        )
        db.commit()
        raise
    finally:
        db.close()