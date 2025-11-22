use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tool_type: String,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub default_params: Option<JsonValue>,
    pub display_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ListToolsQuery {
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateToolRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tool_type: Option<String>,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub default_params: Option<JsonValue>,
    pub display_order: Option<i32>,
}

/// List all tools with optional filtering
pub async fn list_tools(db: &PgPool, params: ListToolsQuery) -> Result<Vec<Tool>> {
    let mut query = "SELECT * FROM app.tools WHERE 1=1".to_string();

    if let Some(category) = params.category {
        query.push_str(&format!(" AND category = '{}'", category));
    }

    query.push_str(" ORDER BY display_order ASC NULLS LAST, name ASC");

    let tools = sqlx::query_as::<_, Tool>(&query)
        .fetch_all(db)
        .await
        .map_err(|e| Error::Database(format!("Failed to fetch tools: {}", e)))?;

    Ok(tools)
}

/// Get a single tool by ID
pub async fn get_tool(db: &PgPool, id: String) -> Result<Tool> {
    let tool = sqlx::query_as::<_, Tool>("SELECT * FROM app.tools WHERE id = $1")
        .bind(&id)
        .fetch_optional(db)
        .await
        .map_err(|e| Error::Database(format!("Failed to fetch tool {}: {}", id, e)))?
        .ok_or_else(|| Error::NotFound(format!("Tool not found: {}", id)))?;

    Ok(tool)
}

/// Update a tool's metadata
pub async fn update_tool(db: &PgPool, id: String, payload: UpdateToolRequest) -> Result<Tool> {
    let mut updates = Vec::new();
    let mut param_count = 1;

    if payload.name.is_some() {
        updates.push(format!("name = ${}", param_count));
        param_count += 1;
    }

    if payload.description.is_some() {
        updates.push(format!("description = ${}", param_count));
        param_count += 1;
    }

    if payload.tool_type.is_some() {
        updates.push(format!("tool_type = ${}", param_count));
        param_count += 1;
    }

    if payload.category.is_some() {
        updates.push(format!("category = ${}", param_count));
        param_count += 1;
    }

    if payload.icon.is_some() {
        updates.push(format!("icon = ${}", param_count));
        param_count += 1;
    }

    if payload.display_order.is_some() {
        updates.push(format!("display_order = ${}", param_count));
        param_count += 1;
    }

    if payload.default_params.is_some() {
        updates.push(format!("default_params = ${}", param_count));
        param_count += 1;
    }

    if updates.is_empty() {
        return Err(Error::InvalidInput("No fields to update".to_string()));
    }

    updates.push("updated_at = NOW()".to_string());

    // Build the query
    let query = format!(
        "UPDATE app.tools SET {} WHERE id = ${} RETURNING *",
        updates.join(", "),
        param_count
    );

    // Start building the query and bind parameters in order
    let mut sqlx_query = sqlx::query_as::<_, Tool>(&query);

    if let Some(name) = payload.name {
        sqlx_query = sqlx_query.bind(name);
    }
    if let Some(description) = payload.description {
        sqlx_query = sqlx_query.bind(description);
    }
    if let Some(tool_type) = payload.tool_type {
        sqlx_query = sqlx_query.bind(tool_type);
    }
    if let Some(category) = payload.category {
        sqlx_query = sqlx_query.bind(category);
    }
    if let Some(icon) = payload.icon {
        sqlx_query = sqlx_query.bind(icon);
    }
    if let Some(display_order) = payload.display_order {
        sqlx_query = sqlx_query.bind(display_order);
    }
    if let Some(default_params) = payload.default_params {
        sqlx_query = sqlx_query.bind(default_params);
    }

    // Bind the ID last
    sqlx_query = sqlx_query.bind(&id);

    let updated_tool = sqlx_query
        .fetch_optional(db)
        .await
        .map_err(|e| Error::Database(format!("Failed to update tool {}: {}", id, e)))?
        .ok_or_else(|| Error::NotFound(format!("Tool not found: {}", id)))?;

    Ok(updated_tool)
}
