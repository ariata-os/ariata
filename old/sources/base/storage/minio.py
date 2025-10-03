"""Unified MinIO storage service for all sources."""

from typing import Any, Optional, List, Dict
from datetime import datetime, timezone
import aioboto3
import os
import json
from pathlib import Path
from uuid import uuid4
from botocore.config import Config

# Create config with connection pooling for better concurrent performance
MINIO_CONFIG = Config(
    max_pool_connections=20,  # 2x our semaphore limit for buffering
    retries={
        'max_attempts': 3,
        'mode': 'adaptive'  # Exponential backoff with jitter
    },
    connect_timeout=10,
    read_timeout=30
)


def get_minio_config() -> Dict[str, Any]:
    """Get MinIO configuration from environment variables."""
    return {
        'endpoint': os.getenv('MINIO_ENDPOINT', 'localhost:9000'),
        'access_key': os.getenv('MINIO_ACCESS_KEY', 'minioadmin'),
        'secret_key': os.getenv('MINIO_SECRET_KEY', 'minioadmin'),
        'bucket': os.getenv('MINIO_BUCKET', 'ariata'),
        'use_ssl': os.getenv('MINIO_USE_SSL', 'false').lower() == 'true',
        'region': os.getenv('MINIO_REGION', 'us-east-1')
    }


class MinIOClient:
    """Async MinIO client for storing and retrieving raw source data."""
    
    def __init__(self):
        # Get MinIO configuration from centralized config
        config = get_minio_config()
        
        self.endpoint_url = config['endpoint']
        if not self.endpoint_url.startswith("http"):
            protocol = "https" if config['use_ssl'] else "http"
            self.endpoint_url = f"{protocol}://{self.endpoint_url}"
        
        self.access_key = config['access_key']
        self.secret_key = config['secret_key']
        self.use_ssl = config['use_ssl']
        self.default_bucket = config['bucket']
        
        self.session = aioboto3.Session()
    
    async def put_raw_data(
        self,
        source_name: str,
        connection_id: str,
        data: bytes,
        filename: str,
        timestamp: Optional[datetime] = None,
        content_type: str = "application/octet-stream",
        bucket: Optional[str] = None
    ) -> str:
        """
        Store raw source data with consistent key structure.
        
        Args:
            source_name: Name of the source (e.g., 'google_calendar')
            connection_id: UUID of the connection
            data: Raw data bytes to store
            filename: Name of the file
            timestamp: Optional timestamp (defaults to now)
            content_type: MIME type of the data
            bucket: Optional bucket name (defaults to configured bucket)
            
        Returns:
            The S3 key where the data was stored
        """
        if timestamp is None:
            timestamp = datetime.utcnow()
        
        # Create consistent key structure: source/date/connection/filename
        # This makes it easier to browse by date and keeps files organized
        key = f"{source_name}/{timestamp.strftime('%Y/%m/%d')}/{connection_id}/{filename}"
        
        await self.put_object(
            bucket=bucket or self.default_bucket,
            key=key,
            data=data,
            content_type=content_type
        )
        
        return key
    
    async def get_raw_data(
        self,
        key: str,
        bucket: Optional[str] = None
    ) -> bytes:
        """
        Retrieve raw data by key.
        
        Args:
            key: S3 key of the object
            bucket: Optional bucket name (defaults to configured bucket)
            
        Returns:
            Raw data bytes
        """
        return await self.get_object(
            bucket=bucket or self.default_bucket,
            key=key
        )
    
    async def list_source_files(
        self,
        source_name: str,
        connection_id: str,
        since: Optional[datetime] = None,
        until: Optional[datetime] = None,
        bucket: Optional[str] = None,
        max_keys: int = 1000
    ) -> List[Dict[str, Any]]:
        """
        List files for a specific source and connection with optional time filtering.
        
        Args:
            source_name: Name of the source
            connection_id: UUID of the connection
            since: Optional start timestamp
            until: Optional end timestamp
            bucket: Optional bucket name (defaults to configured bucket)
            max_keys: Maximum number of keys to return
            
        Returns:
            List of file metadata dictionaries
        """
        prefix = f"{source_name}/{connection_id}/"
        
        # If we have time bounds, we could optimize the prefix
        # For now, we'll list all and filter
        objects = await self.list_objects(
            bucket=bucket or self.default_bucket,
            prefix=prefix,
            max_keys=max_keys
        )
        
        # Filter by time if provided
        if since or until:
            filtered = []
            for obj in objects:
                last_modified = obj.get('LastModified')
                if last_modified:
                    if since and last_modified < since:
                        continue
                    if until and last_modified > until:
                        continue
                    filtered.append(obj)
            return filtered
        
        return objects
    
    async def delete_old_data(
        self,
        source_name: str,
        connection_id: str,
        older_than: datetime,
        bucket: Optional[str] = None
    ) -> int:
        """
        Delete old raw data for cleanup.
        
        Args:
            source_name: Name of the source
            connection_id: UUID of the connection
            older_than: Delete files older than this timestamp
            bucket: Optional bucket name (defaults to configured bucket)
            
        Returns:
            Number of objects deleted
        """
        objects = await self.list_source_files(
            source_name=source_name,
            connection_id=connection_id,
            until=older_than,
            bucket=bucket
        )
        
        if not objects:
            return 0
        
        # Delete objects in batches
        async with self.session.client(
            "s3",
            endpoint_url=self.endpoint_url,
            aws_access_key_id=self.access_key,
            aws_secret_access_key=self.secret_key,
            use_ssl=self.use_ssl
        ) as s3:
            delete_objects = [{"Key": obj["Key"]} for obj in objects]
            
            response = await s3.delete_objects(
                Bucket=bucket or self.default_bucket,
                Delete={"Objects": delete_objects}
            )
            
            return len(response.get("Deleted", []))
    
    # Lower-level methods (migrated from original MinIOStorage)
    
    async def put_object(
        self,
        bucket: str,
        key: str,
        data: bytes,
        content_type: str = "application/octet-stream"
    ) -> None:
        """Store an object in MinIO."""
        async with self.session.client(
            "s3",
            endpoint_url=self.endpoint_url,
            aws_access_key_id=self.access_key,
            aws_secret_access_key=self.secret_key,
            use_ssl=self.use_ssl
        ) as s3:
            await s3.put_object(
                Bucket=bucket,
                Key=key,
                Body=data,
                ContentType=content_type
            )
    
    async def get_object(self, bucket: str, key: str) -> bytes:
        """Retrieve an object from MinIO."""
        async with self.session.client(
            "s3",
            endpoint_url=self.endpoint_url,
            aws_access_key_id=self.access_key,
            aws_secret_access_key=self.secret_key,
            use_ssl=self.use_ssl
        ) as s3:
            response = await s3.get_object(Bucket=bucket, Key=key)
            return await response["Body"].read()
    
    async def list_objects(
        self,
        bucket: str,
        prefix: Optional[str] = None,
        max_keys: int = 1000
    ) -> list:
        """List objects in a bucket with optional prefix."""
        async with self.session.client(
            "s3",
            endpoint_url=self.endpoint_url,
            aws_access_key_id=self.access_key,
            aws_secret_access_key=self.secret_key,
            use_ssl=self.use_ssl
        ) as s3:
            params = {"Bucket": bucket, "MaxKeys": max_keys}
            if prefix:
                params["Prefix"] = prefix
            
            response = await s3.list_objects_v2(**params)
            return response.get("Contents", [])


# Standalone functions for backward compatibility
async def store_raw_data(
    stream_name: str,
    connection_id: str,
    data: Any,
    timestamp: datetime,
    custom_path: Optional[str] = None,
    content_type: str = 'application/json'
) -> str:
    """
    Store raw data to MinIO and return the key.
    
    Args:
        stream_name: Name of the stream
        connection_id: Connection/stream instance ID
        data: Data to store (can be dict for JSON or bytes for binary)
        timestamp: Timestamp for the data
        custom_path: Optional custom path to use instead of auto-generated
        content_type: MIME type of the data
        
    Returns:
        The key where data was stored
    """
    # Use custom path if provided, otherwise generate one
    if custom_path:
        key = custom_path
    else:
        # Generate key based on stream and date
        date_path = timestamp.strftime("%Y/%m/%d")
        file_id = uuid4().hex
        
        # Determine extension based on content type
        extension = '.json'
        if content_type.startswith('audio/'):
            # Use appropriate extension based on content type
            if 'wav' in content_type:
                extension = '.wav'
            elif 'mp4' in content_type or 'aac' in content_type or 'm4a' in content_type:
                extension = '.m4a'
            else:
                extension = '.mp3'
        elif content_type.startswith('image/'):
            extension = '.jpg' if 'jpeg' in content_type else '.png'
        elif content_type == 'application/pdf':
            extension = '.pdf'
        elif isinstance(data, bytes):
            extension = '.bin'
            
        key = f"streams/{stream_name}/{date_path}/{file_id}{extension}"
    
    # Prepare data for storage
    if isinstance(data, bytes):
        # Binary data - store as-is
        body_data = data
    elif isinstance(data, str):
        # String data - encode to bytes
        body_data = data.encode('utf-8')
    else:
        # Convert to JSON
        body_data = json.dumps(data, default=str).encode('utf-8')
    
    # Get MinIO configuration
    config = get_minio_config()
    endpoint = config['endpoint'].replace('http://', '').replace('https://', '')
    
    # Create S3 client with connection pooling config
    session = aioboto3.Session()
    async with session.client(
        's3',
        endpoint_url=f"{'https' if config['use_ssl'] else 'http'}://{endpoint}",
        aws_access_key_id=config['access_key'],
        aws_secret_access_key=config['secret_key'],
        config=MINIO_CONFIG,  # Use our pooled config with retries
        region_name=config['region']
    ) as s3:
        # Upload to MinIO
        await s3.put_object(
            Bucket=config['bucket'],
            Key=key,
            Body=body_data,
            ContentType=content_type,
            Metadata={
                'connection_id': connection_id,
                'stream_name': stream_name,
                'timestamp': timestamp.isoformat()
            }
        )
    
    return key


class HybridStreamStorage:
    """
    Manages the split between PostgreSQL and MinIO storage for stream data.
    
    Rules:
    1. Field destination is predetermined by schema configuration
    2. No runtime size checks - completely deterministic
    3. Always maintain referential integrity
    """
    
    def __init__(self, db_session, minio_client: Optional[MinIOClient] = None, minio_fields: List[str] = None):
        """
        Initialize hybrid storage with database session and field mapping.
        
        Args:
            db_session: SQLAlchemy database session
            minio_client: Optional MinIO client (will create if not provided)
            minio_fields: List of field names that should go to MinIO
        """
        self.db = db_session
        self.minio = minio_client or MinIOClient()
        self.minio_fields = set(minio_fields or [])
    
    def should_use_minio(self, field_name: str) -> bool:
        """
        Determine if field should be stored in MinIO.
        
        This is completely deterministic based on field name,
        not on data size or type.
        
        Args:
            field_name: Name of the field
            
        Returns:
            True if field should go to MinIO, False for PostgreSQL
        """
        return field_name in self.minio_fields
    
    async def store_stream_data(
        self,
        stream_name: str,
        stream_id: str,
        data: Dict[str, Any],
        table_name: str
    ) -> Dict[str, Any]:
        """
        Store stream data using hybrid strategy.
        
        Args:
            stream_name: Name of the stream (e.g., 'ios_mic')
            stream_id: Stream instance ID
            data: Data to store
            table_name: PostgreSQL table name
            
        Returns:
            Dict with stored data references
        """
        from sqlalchemy import text
        
        # Separate data into PostgreSQL and MinIO parts
        postgres_data, minio_refs = await self._split_data(
            data, stream_name, stream_id
        )
        
        # Add MinIO references to PostgreSQL data
        for field_name, minio_path in minio_refs.items():
            # Store the MinIO path in PostgreSQL
            postgres_data[f"{field_name}_path"] = minio_path
            # Store metadata about the asset
            postgres_data[f"{field_name}_stored_at"] = datetime.now(timezone.utc)
        
        # Store in PostgreSQL
        stored_id = self._store_to_postgres(table_name, postgres_data)
        
        return {
            'id': stored_id,
            'postgres_fields': list(postgres_data.keys()),
            'minio_assets': list(minio_refs.keys()),
            'table': table_name
        }
    
    async def _split_data(
        self,
        data: Dict[str, Any],
        stream_name: str,
        stream_id: str
    ) -> tuple[Dict[str, Any], Dict[str, str]]:
        """
        Split data between PostgreSQL and MinIO storage.
        
        Returns:
            Tuple of (postgres_data, minio_references)
        """
        postgres_data = {}
        minio_refs = {}
        
        for field_name, value in data.items():
            if value is None:
                postgres_data[field_name] = None
                continue
            
            if self.should_use_minio(field_name):
                # Store in MinIO and get reference
                minio_path = await self._store_to_minio(
                    value, stream_name, stream_id, field_name
                )
                minio_refs[field_name] = minio_path
            else:
                # Keep in PostgreSQL
                postgres_data[field_name] = value
        
        return postgres_data, minio_refs
    
    async def _store_to_minio(
        self,
        data: Any,
        stream_name: str, 
        stream_id: str,
        field_name: str
    ) -> str:
        """
        Store data in MinIO and return the path.
        
        Returns:
            MinIO object path
        """
        import base64
        
        # Generate consistent path
        timestamp = datetime.now(timezone.utc)
        year = timestamp.strftime('%Y')
        month = timestamp.strftime('%m')
        day = timestamp.strftime('%d')
        
        # Determine file extension
        ext = self._get_extension(data, field_name)
        
        # Generate unique filename
        file_id = str(uuid4())
        filename = f"{field_name}_{file_id}{ext}"
        
        # Build path: assets/{stream_name}/{year}/{month}/{day}/{filename}
        path = f"assets/{stream_name}/{year}/{month}/{day}/{filename}"
        
        # Store to MinIO
        if isinstance(data, str):
            # Assume base64 if string
            data = base64.b64decode(data)
        
        # Use existing MinIO storage function
        await store_raw_data(
            stream_name=stream_name,
            connection_id=stream_id,
            data=data,
            timestamp=timestamp,
            custom_path=path
        )
        
        return path
    
    def _store_to_postgres(self, table_name: str, data: Dict[str, Any]) -> str:
        """
        Store data in PostgreSQL table.
        
        Returns:
            ID of inserted row
        """
        from sqlalchemy import text
        
        # Generate ID if not present
        if 'id' not in data:
            data['id'] = str(uuid4())
        
        # Add timestamps if not present
        now = datetime.now(timezone.utc)
        if 'created_at' not in data:
            data['created_at'] = now
        if 'updated_at' not in data:
            data['updated_at'] = now
        
        # Build INSERT query dynamically
        columns = list(data.keys())
        values_placeholders = [f":{col}" for col in columns]
        
        query = text(f"""
            INSERT INTO {table_name} ({', '.join(columns)})
            VALUES ({', '.join(values_placeholders)})
            RETURNING id
        """)
        
        result = self.db.execute(query, data)
        self.db.commit()
        
        return result.scalar()
    
    async def retrieve_stream_data(
        self,
        table_name: str,
        record_id: str
    ) -> Dict[str, Any]:
        """
        Retrieve stream data from hybrid storage.
        
        Args:
            table_name: PostgreSQL table name
            record_id: Record ID
            
        Returns:
            Complete data with MinIO assets loaded
        """
        from sqlalchemy import text
        
        # Get PostgreSQL data
        query = text(f"SELECT * FROM {table_name} WHERE id = :id")
        result = self.db.execute(query, {"id": record_id})
        row = result.fetchone()
        
        if not row:
            raise ValueError(f"Record {record_id} not found in {table_name}")
        
        # Convert to dict
        data = dict(row)
        
        # Load MinIO assets based on _path fields
        for key in list(data.keys()):
            if key.endswith('_path') and data[key]:
                # This is a MinIO reference
                field_name = key[:-5]  # Remove '_path' suffix
                
                # For now, just note it's available in MinIO
                data[f"{field_name}_in_minio"] = True
        
        return data
    
    def _get_extension(self, data: Any, field_name: str) -> str:
        """Determine file extension based on data and field name."""
        # Common field name to extension mapping
        extension_map = {
            'audio_data': '.m4a',  # iOS sends AAC audio in M4A format
            'image_data': '.jpg',
            'pdf_data': '.pdf',  
            'file_data': '.bin',
        }
        
        if field_name in extension_map:
            return extension_map[field_name]
        
        # Try to detect from data (if it has magic bytes)
        if isinstance(data, bytes):
            if data.startswith(b'RIFF'):
                return '.wav'
            elif data.startswith(b'\xff\xd8\xff'):
                return '.jpg'
            elif data.startswith(b'%PDF'):
                return '.pdf'
        
        # Default to .bin for binary data
        return '.bin'