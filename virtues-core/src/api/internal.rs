//! Boot-time server status.
//!
//! `app_user_profile.server_status` predates pair-only auth: a hosted box sat
//! in `provisioning` until the cloud pushed the owner's profile over
//! `/internal/hydrate`. Nothing calls that any more, and no box waits on it —
//! the row flips to `ready` at boot and stays there. The column survives
//! because `virtues status --json` still reports it.

use crate::error::{Error, Result};
use sqlx::PgPool;

const OWNER_PROFILE_ID: &str = "00000000-0000-0000-0000-000000000001";

/// Ensure `server_status` is `ready` on startup.
///
/// If the row is still `provisioning` at boot, the migration is done and we
/// are ready — flip immediately.
pub async fn ensure_server_status(pool: &PgPool) -> Result<()> {
    let row: std::result::Result<(String,), sqlx::Error> =
        sqlx::query_as("SELECT server_status FROM app_user_profile WHERE id = $1")
            .bind(OWNER_PROFILE_ID)
            .fetch_one(pool)
            .await;

    match row {
        Ok((status,)) if status == "provisioning" => {
            sqlx::query(
                "UPDATE app_user_profile SET server_status = 'ready', updated_at = now() \
                 WHERE id = $1",
            )
            .bind(OWNER_PROFILE_ID)
            .execute(pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to mark server ready: {}", e)))?;
            tracing::info!("Server marked ready on startup");
        }
        Ok(_) => {
            tracing::debug!("Server already in ready state");
        }
        Err(e) => {
            tracing::warn!("Could not check server status: {}", e);
        }
    }

    Ok(())
}
