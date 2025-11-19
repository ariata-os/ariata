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
/// create source connection entries in the database when devices first send data.
///
/// # Arguments
/// * `db` - Database connection
/// * `source_name` - Source name ("ios" or "mac")
/// * `device_id` - Unique device identifier sent by the device
///
/// # Returns
/// The source_connection_id (UUID) for this device, either existing or newly created
///
/// # Example
/// ```
/// let source_id = get_or_create_device_source(db, "ios", "iPhone-12345").await?;
/// ```
pub async fn get_or_create_device_source(
    db: &Database,
    source_name: &str,
    device_id: &str,
) -> Result<Uuid> {
    use sqlx;

    // Try to get existing source connection by device_id
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM source_connections WHERE source = $1 AND device_id = $2 AND auth_type = 'device'"
    )
    .bind(source_name)
    .bind(device_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Failed to query source connection: {e}")))?;

    if let Some((id,)) = existing {
        return Ok(id);
    }

    // Create new device source connection with a unique name
    let new_id = Uuid::new_v4();
    let name = format!("{}-{}", source_name, device_id);

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO source_connections (id, source, name, device_id, auth_type, is_active, is_internal)
         VALUES ($1, $2, $3, $4, 'device', true, false)
         ON CONFLICT (source, device_id) WHERE device_id IS NOT NULL
         DO UPDATE SET updated_at = NOW()
         RETURNING id",
    )
    .bind(new_id)
    .bind(source_name)
    .bind(&name)
    .bind(device_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Failed to create source connection: {e}")))?;

    Ok(id)
}
