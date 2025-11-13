//! Job data models and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Job type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum JobType {
    Sync,
    Transform,
    Archive,
}

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobType::Sync => write!(f, "sync"),
            JobType::Transform => write!(f, "transform"),
            JobType::Archive => write!(f, "archive"),
        }
    }
}

impl std::str::FromStr for JobType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sync" => Ok(JobType::Sync),
            "transform" => Ok(JobType::Transform),
            "archive" => Ok(JobType::Archive),
            _ => Err(format!("Invalid job type: {}", s)),
        }
    }
}

/// Job status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Succeeded => write!(f, "succeeded"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(JobStatus::Pending),
            "running" => Ok(JobStatus::Running),
            "succeeded" => Ok(JobStatus::Succeeded),
            "failed" => Ok(JobStatus::Failed),
            "cancelled" => Ok(JobStatus::Cancelled),
            _ => Err(format!("Invalid job status: {}", s)),
        }
    }
}

/// Job model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Job {
    pub id: Uuid,
    pub job_type: JobType,
    pub status: JobStatus,

    // Sync job fields
    pub source_id: Option<Uuid>,
    pub stream_name: Option<String>,
    pub sync_mode: Option<String>,  // 'full_refresh' or 'incremental'

    // Transform job fields
    pub transform_id: Option<Uuid>,
    pub transform_strategy: Option<String>,

    // Job chaining
    pub parent_job_id: Option<Uuid>,
    pub transform_stage: Option<String>,

    // Tracking
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub records_processed: i64,
    pub error_message: Option<String>,
    pub error_class: Option<String>,

    // Metadata
    pub metadata: serde_json::Value,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Custom TryFrom implementations for sqlx type conversion
impl TryFrom<String> for JobType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for JobStatus {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// Request to create a new job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub job_type: JobType,
    pub status: JobStatus,

    // Sync job fields
    pub source_id: Option<Uuid>,
    pub stream_name: Option<String>,
    pub sync_mode: Option<String>,  // 'full_refresh' or 'incremental'

    // Transform job fields
    pub transform_id: Option<Uuid>,
    pub transform_strategy: Option<String>,

    // Job chaining
    pub parent_job_id: Option<Uuid>,
    pub transform_stage: Option<String>,

    // Metadata
    pub metadata: serde_json::Value,
}

/// Metadata for sync jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJobMetadata {
    pub sync_mode: String,
    pub cursor_before: Option<String>,
}

impl CreateJobRequest {
    /// Create a request for a new sync job
    pub fn new_sync_job(
        source_id: Uuid,
        stream_name: String,
        sync_mode: String,  // 'full_refresh' or 'incremental'
        metadata: SyncJobMetadata,
    ) -> Self {
        Self {
            job_type: JobType::Sync,
            status: JobStatus::Pending,
            source_id: Some(source_id),
            stream_name: Some(stream_name),
            sync_mode: Some(sync_mode),
            transform_id: None,
            transform_strategy: None,
            parent_job_id: None,
            transform_stage: None,
            metadata: serde_json::to_value(metadata).unwrap_or_default(),
        }
    }

    /// Create a request for a new transform job
    pub fn new_transform_job(transform_id: Uuid, transform_strategy: String) -> Self {
        Self {
            job_type: JobType::Transform,
            status: JobStatus::Pending,
            source_id: None,
            stream_name: None,
            sync_mode: None,
            transform_id: Some(transform_id),
            transform_strategy: Some(transform_strategy),
            parent_job_id: None,
            transform_stage: None,
            metadata: serde_json::json!({}),
        }
    }
}
