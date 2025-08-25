"""
Base Stream Processor for Jaces
================================

This module provides the base class for all stream processors.
Stream processors handle the transformation and storage of raw data
using a hybrid storage strategy (PostgreSQL + MinIO).
"""

from abc import ABC, abstractmethod
from typing import Any, Dict, List, Optional, Type, Union
from datetime import datetime, timezone
from pathlib import Path
import yaml
import asyncio
from pydantic import BaseModel
from sqlalchemy import text

from sources.base.storage.minio import HybridStreamStorage


class StreamProcessor(ABC):
    """
    Abstract base class for stream processors using hybrid storage.
    
    Automatically loads configuration from _stream.yaml and provides
    common functionality for all processors.
    
    Subclasses must:
    1. Override process_record() to handle specific stream types
    2. Optionally override validate_record() for custom validation
    """
    
    def __init__(self):
        """
        Initialize processor by auto-loading configuration from _stream.yaml.
        """
        # Derive paths from class name
        # e.g., GoogleCalendarStreamProcessor -> google/calendar
        class_name = self.__class__.__name__
        if not class_name.endswith('StreamProcessor'):
            raise ValueError(f"Processor class must end with 'StreamProcessor': {class_name}")
        
        # Remove 'StreamProcessor' suffix and split by capitals
        name_parts = class_name[:-15]  # Remove 'StreamProcessor'
        
        # Convert CamelCase to snake_case components
        import re
        parts = re.findall('[A-Z][a-z]+', name_parts)
        if len(parts) < 2:
            raise ValueError(f"Cannot derive source/stream from class name: {class_name}")
        
        source_name = parts[0].lower()
        stream_suffix = '_'.join(p.lower() for p in parts[1:])
        
        # Load configuration from _stream.yaml
        config_path = Path(f'/sources/{source_name}/{stream_suffix}/_stream.yaml')
        if not config_path.exists():
            raise ValueError(f"Stream configuration not found: {config_path}")
        
        with open(config_path) as f:
            self.config = yaml.safe_load(f)
        
        # Extract key configuration
        self.stream_name = self.config.get('name')
        self.source_name = self.config.get('source')
        
        # Get processor config
        processor_config = self.config.get('processor', {})
        self.table_name = processor_config.get('table_name', f'stream_{self.stream_name}')
        self.minio_fields = processor_config.get('minio_fields', [])
        
        # Get storage config
        storage_config = self.config.get('storage', {})
        self.storage_strategy = storage_config.get('strategy', 'postgres_only')
        
        # Override minio_fields from storage config if specified
        if 'minio_fields' in storage_config:
            self.minio_fields = storage_config.get('minio_fields', [])
    
    @property
    @abstractmethod
    def model_class(self) -> Type[BaseModel]:
        """
        Return the Pydantic model class for this stream.
        
        Must be overridden in subclasses to specify the model.
        
        Example:
            @property
            def model_class(self):
                from .models import StreamGoogleCalendar
                return StreamGoogleCalendar
        """
        pass
    
    async def process_batch(
        self,
        records: Union[Dict[str, Any], List[Dict[str, Any]]],
        source_id: str
    ) -> Dict[str, Any]:
        """
        Process a batch of records using Pydantic models.
        
        Args:
            records: Single record dict or list of records to process
            source_id: Source instance ID
            
        Returns:
            Processing summary
        """
        from sources.base.storage.database import SyncSessionLocal
        
        db_session = SyncSessionLocal()
        try:
            # Handle the ingestion data format vs direct records
            if isinstance(records, dict) and 'data' in records:
                # Ingestion format: {'stream_name': ..., 'data': [...], 'batch_metadata': ...}
                records_list = records['data']
                print(f"[StreamProcessor] Processing {len(records_list)} records from ingestion batch for {self.stream_name}")
            elif isinstance(records, dict):
                # Single record - wrap in list
                records_list = [records]
                print(f"[StreamProcessor] Processing single record for {self.stream_name}")
            else:
                # List of records
                records_list = records
                print(f"[StreamProcessor] Processing {len(records_list)} records for {self.stream_name}")
            
            # Process all records
            processed_models = []
            for record in records_list:
                # Validate record
                if not self.validate_record(record):
                    continue
                
                # Process individual record
                processed_data = await self.process_record(record)
                
                # Add common fields
                processed_data['source_id'] = source_id
                processed_data['timestamp'] = processed_data.get('timestamp', datetime.now(timezone.utc))
                processed_data['created_at'] = datetime.now(timezone.utc)
                processed_data['updated_at'] = datetime.now(timezone.utc)
                
                # Create and validate with Pydantic model
                try:
                    # Create model instance excluding auto-managed fields for validation
                    validation_data = processed_data.copy()
                    # Remove auto-managed fields that aren't in the model
                    for field in ['id', 'source_id', 'timestamp', 'created_at', 'updated_at']:
                        validation_data.pop(field, None)
                    
                    model_instance = self.model_class(**validation_data)
                    
                    # Store the full processed_data for database insertion
                    # We'll use processed_data (not model) for the actual insert
                    processed_models.append((model_instance, processed_data))
                except Exception as e:
                    print(f"Validation error for record: {e}")
                    continue
            
            # Bulk insert using processed data with MinIO handling
            minio_assets_count = 0
            if processed_models:
                # Extract the processed_data from tuples
                records_for_db = []
                
                # Check if we need hybrid storage
                if self.minio_fields:
                    # Use HybridStreamStorage for MinIO fields
                    from sources.base.storage.minio import store_raw_data
                    import base64
                    import asyncio
                    from uuid import uuid4
                    
                    # Prepare MinIO upload tasks
                    minio_tasks = []
                    record_indices = []
                    
                    for idx, (model_instance, processed_data) in enumerate(processed_models):
                        # Split data between PostgreSQL and MinIO
                        postgres_record = {}
                        has_minio_field = False
                        
                        for field_name, value in processed_data.items():
                            if field_name in self.minio_fields and value is not None:
                                has_minio_field = True
                                # Prepare MinIO upload
                                try:
                                    # Decode base64 if it's audio data
                                    if isinstance(value, str) and field_name == 'audio_data':
                                        binary_data = base64.b64decode(value)
                                    else:
                                        binary_data = value
                                    
                                    # Generate MinIO path
                                    timestamp = processed_data.get('timestamp', datetime.now(timezone.utc))
                                    
                                    # Create task for concurrent upload
                                    task = store_raw_data(
                                        stream_name=self.stream_name,
                                        connection_id=processed_data['source_id'],
                                        data=binary_data,
                                        timestamp=timestamp,
                                        content_type='audio/wav' if field_name == 'audio_data' else 'application/octet-stream'
                                    )
                                    minio_tasks.append(task)
                                    record_indices.append(idx)
                                    
                                    # Placeholder for MinIO path
                                    postgres_record['file_size'] = len(binary_data) if isinstance(binary_data, bytes) else 0
                                    
                                except Exception as e:
                                    print(f"[StreamProcessor] Failed to prepare {field_name} for MinIO: {e}")
                                    # Continue without the field
                            else:
                                # Keep in PostgreSQL
                                postgres_record[field_name] = value
                        
                        records_for_db.append(postgres_record)
                    
                    # Execute MinIO uploads with controlled concurrency using semaphore
                    if minio_tasks:
                        print(f"[StreamProcessor] Uploading {len(minio_tasks)} assets to MinIO (max 10 concurrent)...")
                        
                        # Create semaphore to limit concurrent uploads
                        semaphore = asyncio.Semaphore(25)
                        
                        # Track progress
                        completed = 0
                        batch_size = 50  # Log every 50 uploads
                        
                        # Wrapper to apply semaphore and track progress
                        async def upload_with_semaphore(task, idx):
                            nonlocal completed
                            async with semaphore:
                                try:
                                    result = await task
                                    completed += 1
                                    if completed % batch_size == 0 or completed == len(minio_tasks):
                                        print(f"[StreamProcessor] MinIO upload progress: {completed}/{len(minio_tasks)}")
                                    return result
                                except Exception as e:
                                    print(f"[StreamProcessor] MinIO upload failed for record {idx}: {e}")
                                    completed += 1
                                    return e
                        
                        # Create limited tasks with progress tracking
                        limited_tasks = [
                            upload_with_semaphore(task, idx) 
                            for task, idx in zip(minio_tasks, record_indices)
                        ]
                        
                        # Execute with controlled concurrency
                        minio_results = await asyncio.gather(*limited_tasks, return_exceptions=True)
                        
                        # Update records with MinIO paths
                        for idx, result in zip(record_indices, minio_results):
                            if isinstance(result, Exception):
                                print(f"[StreamProcessor] Skipping MinIO path for record {idx} due to upload failure")
                            else:
                                records_for_db[idx]['minio_path'] = result
                                minio_assets_count += 1
                        
                        print(f"[StreamProcessor] Successfully uploaded {minio_assets_count}/{len(minio_tasks)} assets to MinIO")
                else:
                    # No MinIO fields, use direct insert
                    for model_instance, processed_data in processed_models:
                        records_for_db.append(processed_data)
                
                # Build bulk insert SQL
                if records_for_db:
                    self._bulk_insert(db_session, records_for_db)
                    db_session.commit()
            
            return {
                'records_processed': len(processed_models),
                'records_stored': len(processed_models),
                'table': self.table_name,
                'minio_assets': minio_assets_count if self.minio_fields else 0
            }
        except Exception as e:
            db_session.rollback()
            raise e
        finally:
            db_session.close()
    
    def _bulk_insert(self, db_session, records: List[Dict[str, Any]]):
        """
        Perform bulk insert using raw SQL.
        
        Args:
            db_session: Database session
            records: List of record dictionaries
        """
        if not records:
            return
        
        import json
        
        # Execute for each record individually to handle different field sets
        for record in records:
            # Get columns for this specific record
            columns = list(record.keys())
            columns_str = ', '.join(columns)
            placeholders = ', '.join([f':{col}' for col in columns])
            
            sql = f"INSERT INTO {self.table_name} ({columns_str}) VALUES ({placeholders})"
            
            # Convert dict and list values to JSON strings for PostgreSQL JSONB columns
            processed_record = {}
            for key, value in record.items():
                if isinstance(value, (dict, list)):
                    processed_record[key] = json.dumps(value)
                else:
                    processed_record[key] = value
            
            db_session.execute(text(sql), processed_record)
    
    @abstractmethod
    async def process_record(self, record: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process a single record.
        
        Must be overridden in subclasses to implement stream-specific logic.
        
        Args:
            record: Raw record data
            
        Returns:
            Processed record ready for storage
        """
        pass
    
    def validate_record(self, record: Dict[str, Any]) -> bool:
        """
        Validate a record before processing.
        
        Override this for custom validation logic.
        
        Args:
            record: Record to validate
            
        Returns:
            True if valid, False otherwise
        """
        return record is not None and isinstance(record, dict)