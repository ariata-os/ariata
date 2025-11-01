

//! Ontology transformation trait and types
//!
//! Defines the interface for transforming raw stream data into normalized ontology tables.
//! Transformations are idempotent and track progress for incremental processing.

use async_trait::async_trait;
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;

/// Result of a transformation operation
#[derive(Debug, Clone)]
pub struct TransformResult {
    /// Number of source records read/examined
    pub records_read: usize,

    /// Number of ontology records successfully written
    pub records_written: usize,

    /// Number of records that failed transformation
    pub records_failed: usize,

    /// ID of last successfully processed source record (for cursor-based iteration)
    pub last_processed_id: Option<Uuid>,
}

/// Trait for transforming stream data into ontology tables
///
/// Each source stream (e.g., Gmail, iMessage) implements this trait to define
/// how its data maps to normalized ontology tables (e.g., social_email).
///
/// # Example
///
/// ```ignore
/// use async_trait::async_trait;
/// use crate::sources::base::{OntologyTransform, TransformResult};
///
/// pub struct GmailEmailTransform;
///
/// #[async_trait]
/// impl OntologyTransform for GmailEmailTransform {
///     fn source_table(&self) -> &str { "stream_google_gmail" }
///     fn target_table(&self) -> &str { "social_email" }
///     fn domain(&self) -> &str { "social" }
///
///     async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult> {
///         // Read from stream_google_gmail
///         // Transform to social_email schema
///         // Write to social_email table
///         // Return stats
///     }
/// }
/// ```
#[async_trait]
pub trait OntologyTransform: Send + Sync {
    /// Source stream table name (e.g., "stream_google_gmail")
    fn source_table(&self) -> &str;

    /// Target ontology table name (e.g., "social_email")
    fn target_table(&self) -> &str;

    /// Domain of the ontology (e.g., "social", "health", "activity")
    fn domain(&self) -> &str;

    /// Transform records from source stream to ontology table
    ///
    /// This method should:
    /// 1. Query source table for untransformed records
    /// 2. Map fields from source schema to ontology schema
    /// 3. Insert into ontology table with source_stream_id reference
    /// 4. Handle errors gracefully and continue processing
    /// 5. Return statistics about the transformation
    ///
    /// **Idempotency**: This should be safe to re-run. Use `ON CONFLICT DO NOTHING`
    /// or left joins to avoid processing already-transformed records.
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection
    /// * `source_id` - UUID of the source (for filtering source table records)
    ///
    /// # Returns
    ///
    /// Result containing transformation statistics
    async fn transform(&self, db: &Database, source_id: Uuid) -> Result<TransformResult>;
}
