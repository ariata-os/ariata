//! API functions for agent management

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::Result;

/// Agent information returned by API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub enabled: bool,
    pub sort_order: i32,
}

/// List all system default agents (user_id IS NULL)
pub async fn list_agents(db: &PgPool) -> Result<Vec<AgentInfo>> {
    let agents = sqlx::query_as!(
        AgentInfo,
        r#"
        SELECT
            agent_id,
            name,
            description,
            color,
            icon,
            enabled,
            sort_order
        FROM app.agents
        WHERE user_id IS NULL AND enabled = true
        ORDER BY sort_order ASC
        "#
    )
    .fetch_all(db)
    .await?;

    Ok(agents)
}

/// Get a specific agent by ID
pub async fn get_agent(db: &PgPool, agent_id: &str) -> Result<AgentInfo> {
    let agent = sqlx::query_as!(
        AgentInfo,
        r#"
        SELECT
            agent_id,
            name,
            description,
            color,
            icon,
            enabled,
            sort_order
        FROM app.agents
        WHERE user_id IS NULL AND agent_id = $1
        "#,
        agent_id
    )
    .fetch_one(db)
    .await?;

    Ok(agent)
}
