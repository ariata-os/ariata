"""
Type definitions for YAML configurations using Pydantic.
Provides runtime validation and type safety for source, stream, and schema configurations.
"""

from typing import List, Optional, Dict, Any, Union, Literal
from enum import Enum
from pydantic import BaseModel, Field, validator, HttpUrl


# Enums for constrained values
class PlatformType(str, Enum):
    CLOUD = "cloud"
    DEVICE = "device"
    HYBRID = "hybrid"


class AuthType(str, Enum):
    OAUTH2 = "oauth2"
    DEVICE_TOKEN = "device_token"


class SyncStrategy(str, Enum):
    TOKEN = "token"
    DATE_RANGE = "date_range"
    CURSOR = "cursor"
    NONE = "none"


class StorageStrategy(str, Enum):
    POSTGRES_ONLY = "postgres_only"
    MINIO_ONLY = "minio_only"
    HYBRID = "hybrid"


class CompressionType(str, Enum):
    NONE = "none"
    GZIP = "gzip"
    BROTLI = "brotli"
    OPUS = "opus"


class DataType(str, Enum):
    STRING = "string"
    TEXT = "text"
    INTEGER = "integer"
    BIGINT = "bigint"
    FLOAT = "float"
    DOUBLE = "double"
    DECIMAL = "decimal"
    BOOLEAN = "boolean"
    TIMESTAMP = "timestamp"
    DATE = "date"
    TIME = "time"
    JSON = "json"
    JSONB = "jsonb"
    UUID = "uuid"
    ARRAY = "array"
    BINARY = "binary"


# OAuth2 Authentication Model
class OAuth2Auth(BaseModel):
    type: Literal["oauth2"]
    provider: Optional[str] = None
    auth_url: Optional[HttpUrl] = None
    token_url: Optional[HttpUrl] = None
    revoke_url: Optional[HttpUrl] = None
    auth_proxy: Optional[HttpUrl] = None
    token_refresh_window: Optional[int] = Field(None, ge=0)
    scopes: Optional[List[str]] = None


# Device Token Authentication Model
class DeviceTokenAuth(BaseModel):
    type: Literal["device_token"]
    required_fields: List[str]


# Stream Definition within Source
class StreamDefinition(BaseModel):
    name: str = Field(..., pattern="^[a-z][a-z0-9_]*$")
    display_name: str
    description: str
    required_scopes: Optional[List[str]] = None


# API Configuration
class APIConfig(BaseModel):
    base_url: Optional[HttpUrl] = None
    endpoint: Optional[HttpUrl] = None
    version: Optional[str] = None
    rate_limit: Optional[int] = Field(None, ge=0)
    rate_limits: Optional[Dict[str, int]] = None
    timeout: Optional[int] = Field(None, ge=0)
    pagination: Optional[bool] = None
    max_results: Optional[int] = Field(None, ge=1)
    page_size: Optional[int] = Field(None, ge=1)


# Sync Configuration
class SyncConfig(BaseModel):
    type: Optional[str] = None
    schedule: Optional[str] = None
    batch_upload_interval: Optional[int] = Field(None, ge=0)
    max_batch_size: Optional[int] = Field(None, ge=1)
    page_size: Optional[int] = Field(None, ge=1)
    compression: Optional[str] = None
    lookback_days: Optional[int] = Field(None, ge=0)
    initial_sync_days: Optional[int] = Field(None, ge=0)
    retry_policy: Optional[Dict[str, Any]] = None
    local_buffer: Optional[Dict[str, Any]] = None


# Source Configuration (_source.yaml)
class SourceConfig(BaseModel):
    name: str = Field(..., pattern="^[a-z][a-z0-9_]*$")
    display_name: str
    icon: Optional[str] = None
    video: Optional[str] = None
    platform: str
    company: Optional[str] = None
    description: str
    auth: Union[OAuth2Auth, DeviceTokenAuth, Dict[str, Any]]
    streams: Optional[List[StreamDefinition]] = None
    api: Optional[APIConfig] = None
    requirements: Optional[Dict[str, Any]] = None
    sync: Optional[SyncConfig] = None
    agent: Optional[Dict[str, Any]] = None
    monitoring: Optional[Dict[str, Any]] = None


# Stream Processor Configuration
class ProcessorConfig(BaseModel):
    # class_name and module are now optional - derived by convention if not specified
    class_name: Optional[str] = None
    module: Optional[str] = None
    table_name: str = Field(..., pattern="^stream_[a-z][a-z0-9_]*$")
    minio_fields: Optional[List[str]] = None


# Processing Configuration
class ProcessingConfig(BaseModel):
    normalization: Optional[bool] = None
    deduplication: Optional[Union[bool, Dict[str, Any]]] = None
    extract_text: Optional[bool] = None
    extract_metadata: Optional[bool] = None
    versioning: Optional[bool] = None
    validation: Optional[Dict[str, Any]] = None


# Storage Configuration
class StorageConfig(BaseModel):
    retention_days: Optional[int] = Field(None, ge=0)
    compression: Optional[str] = None
    format: Optional[str] = None
    strategy: Optional[str] = None
    minio_fields: Optional[List[str]] = None
    note: Optional[str] = None


# Stream Configuration (_stream.yaml)
class StreamConfig(BaseModel):
    name: str = Field(..., pattern="^[a-z][a-z0-9_]*$")
    source: str = Field(..., pattern="^[a-z][a-z0-9_]*$")
    display_name: str
    description: str
    ingestion: Optional[Dict[str, Any]] = None
    api: Optional[APIConfig] = None
    sync: Optional[SyncConfig] = None
    processor: ProcessorConfig
    processing: Optional[ProcessingConfig] = None
    storage: Optional[StorageConfig] = None


# Column Definition for Table Schema
class ColumnDefinition(BaseModel):
    name: str = Field(..., pattern="^[a-z][a-z0-9_]*$")
    type: str
    max_length: Optional[int] = Field(None, ge=1)
    nullable: bool = True
    default: Optional[Union[str, int, float, bool, None]] = None
    description: Optional[str] = None
    index: Optional[bool] = False
    unique: Optional[bool] = False
    primary_key: Optional[bool] = False
    minio_storage: Optional[bool] = False


# Table Schema Configuration (schema.yaml)
class TableSchema(BaseModel):
    table_name: str = Field(..., pattern="^stream_[a-z][a-z0-9_]*$")
    description: Optional[str] = None
    columns: List[ColumnDefinition]
    indexes: Optional[List[Dict[str, Any]]] = None
    storage: Optional[StorageConfig] = None


# Validation functions
def validate_source_config(data: Dict[str, Any]) -> SourceConfig:
    """Validate a source configuration dictionary."""
    return SourceConfig(**data)


def validate_stream_config(data: Dict[str, Any]) -> StreamConfig:
    """Validate a stream configuration dictionary."""
    return StreamConfig(**data)


def validate_table_schema(data: Dict[str, Any]) -> TableSchema:
    """Validate a table schema dictionary."""
    return TableSchema(**data)