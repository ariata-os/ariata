// ! API functions for model management

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::Result;

/// Model information returned by API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub model_id: String,
    pub display_name: String,
    pub provider: String,
    pub enabled: bool,
    pub sort_order: i32,
    pub context_window: Option<i32>,
    pub max_output_tokens: Option<i32>,
    pub supports_tools: Option<bool>,
    pub is_default: Option<bool>,
}

/// List all system default models (user_id IS NULL)
pub async fn list_models(db: &PgPool) -> Result<Vec<ModelInfo>> {
    let models = sqlx::query_as!(
        ModelInfo,
        r#"
        SELECT
            model_id,
            display_name,
            provider,
            enabled,
            sort_order,
            context_window,
            max_output_tokens,
            supports_tools,
            is_default
        FROM app.models
        WHERE user_id IS NULL AND enabled = true
        ORDER BY sort_order ASC
        "#
    )
    .fetch_all(db)
    .await?;

    Ok(models)
}

/// Get a specific model by ID
pub async fn get_model(db: &PgPool, model_id: &str) -> Result<ModelInfo> {
    let model = sqlx::query_as!(
        ModelInfo,
        r#"
        SELECT
            model_id,
            display_name,
            provider,
            enabled,
            sort_order,
            context_window,
            max_output_tokens,
            supports_tools,
            is_default
        FROM app.models
        WHERE user_id IS NULL AND model_id = $1
        "#,
        model_id
    )
    .fetch_one(db)
    .await?;

    Ok(model)
}
