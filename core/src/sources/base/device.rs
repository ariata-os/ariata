//! Device source utilities for push-based data ingestion
//!
//! Provides shared helpers for iOS and Mac device sources that push data
//! to the ingestion endpoint.

use crate::{
    database::Database,
    error::{Error, Result},
};
use uuid::Uuid;

/// Get or create a device source by device_id
///
/// This helper is used by all device processors (iOS, Mac) to automatically
/// create source entries in the database when devices first send data.
///
/// # Arguments
/// * `db` - Database connection
/// * `provider` - Provider name ("ios" or "mac")
/// * `device_id` - Unique device identifier sent by the device
///
/// # Returns
/// The source_id (UUID) for this device, either existing or newly created
///
/// # Example
/// ```
/// let source_id = get_or_create_device_source(db, "ios", "iPhone-12345").await?;
/// ```
pub async fn get_or_create_device_source(
    db: &Database,
    provider: &str,
    device_id: &str,
) -> Result<Uuid> {
    use sqlx;

    // Try to get existing source
    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM sources WHERE provider = $1 AND name = $2")
            .bind(provider)
            .bind(device_id)
            .fetch_optional(db.pool())
            .await
            .map_err(|e| Error::Database(format!("Failed to query source: {e}")))?;

    if let Some((id,)) = existing {
        return Ok(id);
    }

    // Create new device source
    let new_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO sources (id, provider, name, is_active, is_internal)
         VALUES ($1, $2, $3, true, false)
         ON CONFLICT (name) DO NOTHING",
    )
    .bind(new_id)
    .bind(provider)
    .bind(device_id)
    .execute(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Failed to create source: {e}")))?;

    Ok(new_id)
}
