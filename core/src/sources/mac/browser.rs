//! macOS browser history data processor

use serde_json::Value;
use std::sync::Arc;

use crate::{
    database::Database,
    error::{Error, Result},
    sources::base::get_or_create_device_source,
    storage::Storage,
};

/// Process Mac browser history data
///
/// Parses and stores URLs, page titles, and visit durations from Safari, Chrome, and Firefox.
///
/// # Arguments
/// * `db` - Database connection
/// * `_storage` - Storage layer (unused for browser, but kept for API consistency)
/// * `record` - JSON record from the device
#[tracing::instrument(skip(db, _storage, record), fields(source = "mac", stream = "browser"))]
pub async fn process(db: &Database, _storage: &Arc<Storage>, record: &Value) -> Result<()> {
    let device_id = record.get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing device_id".into()))?;

    let source_id = get_or_create_device_source(db, "mac", device_id).await?;

    let timestamp = record.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing timestamp".into()))?;

    let url = record.get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("Missing url".into()))?;

    let title = record.get("title").and_then(|v| v.as_str());
    let domain = record.get("domain").and_then(|v| v.as_str());
    let browser = record.get("browser").and_then(|v| v.as_str());
    let visit_duration = record.get("visit_duration").and_then(|v| v.as_i64()).map(|v| v as i32);
    let transition_type = record.get("transition_type").and_then(|v| v.as_str());

    sqlx::query(
        "INSERT INTO stream_mac_browser
         (source_id, timestamp, url, title, domain, browser, visit_duration,
          transition_type, raw_data)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (source_id, url, timestamp)
         DO UPDATE SET
            title = EXCLUDED.title,
            domain = EXCLUDED.domain,
            browser = EXCLUDED.browser,
            visit_duration = EXCLUDED.visit_duration,
            transition_type = EXCLUDED.transition_type,
            raw_data = EXCLUDED.raw_data"
    )
    .bind(source_id)
    .bind(timestamp)
    .bind(url)
    .bind(title)
    .bind(domain)
    .bind(browser)
    .bind(visit_duration)
    .bind(transition_type)
    .bind(record)
    .execute(db.pool())
    .await
    .map_err(|e| Error::Database(format!("Failed to insert browser data: {e}")))?;

    tracing::debug!("Inserted browser record for device {}", device_id);
    Ok(())
}
