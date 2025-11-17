//! Assistant profile API
//!
//! This module provides functions for managing the user's AI assistant preferences.
//! The assistant profile is a singleton table containing AI/agent configuration.

use crate::error::{Error, Result};
use crate::storage::models::AssistantProfile;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Request to update assistant profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAssistantProfileRequest {
    pub assistant_name: Option<String>,
    pub default_agent_id: Option<String>,
    pub default_model_id: Option<String>,
    pub enabled_tools: Option<serde_json::Value>,
    pub pinned_tool_ids: Option<Vec<String>>,
    pub ui_preferences: Option<serde_json::Value>,
}

/// Get the assistant profile (singleton row)
///
/// This will always return a profile, as the migration creates an empty row by default.
pub async fn get_assistant_profile(db: &PgPool) -> Result<AssistantProfile> {
    let profile = sqlx::query_as::<_, AssistantProfile>(
        r#"
        SELECT *
        FROM data.assistant_profile
        LIMIT 1
        "#,
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to fetch assistant profile: {}", e)))?;

    Ok(profile)
}

/// Update the assistant profile
///
/// Only updates fields that are present in the request (not None).
/// Returns the updated profile.
pub async fn update_assistant_profile(
    db: &PgPool,
    request: UpdateAssistantProfileRequest,
) -> Result<AssistantProfile> {
    // The singleton UUID
    let profile_id =
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").expect("Valid UUID constant");

    // Build dynamic UPDATE query based on which fields are present
    let mut updates = Vec::new();
    let mut query = "UPDATE data.assistant_profile SET ".to_string();

    if request.assistant_name.is_some() {
        updates.push("assistant_name = $1");
    }
    if request.default_agent_id.is_some() {
        updates.push("default_agent_id = $2");
    }
    if request.default_model_id.is_some() {
        updates.push("default_model_id = $3");
    }
    if request.enabled_tools.is_some() {
        updates.push("enabled_tools = $4");
    }
    if request.pinned_tool_ids.is_some() {
        updates.push("pinned_tool_ids = $5");
    }
    if request.ui_preferences.is_some() {
        updates.push("ui_preferences = $6");
    }

    if updates.is_empty() {
        // No updates requested, just return current profile
        return get_assistant_profile(db).await;
    }

    query.push_str(&updates.join(", "));
    query.push_str(", updated_at = NOW() WHERE id = $7 RETURNING *");

    // Execute the update with bound parameters
    let updated_profile = sqlx::query_as::<_, AssistantProfile>(&query)
        .bind(&request.assistant_name)
        .bind(&request.default_agent_id)
        .bind(&request.default_model_id)
        .bind(&request.enabled_tools)
        .bind(&request.pinned_tool_ids)
        .bind(&request.ui_preferences)
        .bind(profile_id)
        .fetch_one(db)
        .await
        .map_err(|e| Error::Database(format!("Failed to update assistant profile: {}", e)))?;

    Ok(updated_profile)
}

/// Helper to get the assistant's name for system prompts
///
/// Returns assistant_name if set, otherwise "Assistant"
pub async fn get_assistant_name(db: &PgPool) -> Result<String> {
    let profile = get_assistant_profile(db).await?;

    Ok(profile
        .assistant_name
        .unwrap_or_else(|| "Assistant".to_string()))
}

/// Get pinned tools with full metadata
///
/// Returns tools from app.tools table filtered by the user's pinned_tool_ids,
/// ordered by the pinned_tool_ids array order
pub async fn get_pinned_tools(db: &PgPool) -> Result<Vec<crate::api::tools::Tool>> {
    use crate::api::tools::Tool;

    let profile = get_assistant_profile(db).await?;
    let pinned_ids = profile.pinned_tool_ids.unwrap_or_default();

    if pinned_ids.is_empty() {
        return Ok(vec![]);
    }

    // Query tools matching the pinned IDs
    let tools = sqlx::query_as::<_, Tool>(
        r#"
        SELECT *
        FROM app.tools
        WHERE id = ANY($1)
        ORDER BY array_position($1, id)
        "#,
    )
    .bind(&pinned_ids)
    .fetch_all(db)
    .await
    .map_err(|e| Error::Database(format!("Failed to fetch pinned tools: {}", e)))?;

    Ok(tools)
}
