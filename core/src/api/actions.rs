//! Actions API - Managing temporal pursuits across different time horizons
//!
//! This module provides CRUD operations for:
//! - Tasks: Daily/weekly completable items
//! - Initiatives: Medium-term commitments (month-quarter)
//! - Aspirations: Long-term life goals (multi-year)
//!
//! These represent the "doing" layer - concrete actions across different time scales.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub topic_id: Option<Uuid>,
    pub status: Option<String>,
    pub progress_percent: Option<i32>,
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    pub target_date: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_date: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub topic_id: Option<Uuid>,
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    pub target_date: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub topic_id: Option<Uuid>,
    pub status: Option<String>,
    pub progress_percent: Option<i32>,
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    pub target_date: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_date: Option<chrono::DateTime<chrono::Utc>>,
}

// Initiative type (same structure as Task)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Initiative {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub topic_id: Option<Uuid>,
    pub status: Option<String>,
    pub progress_percent: Option<i32>,
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    pub target_date: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_date: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// Aspiration type (different lifecycle)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Aspiration {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub topic_id: Option<Uuid>,
    pub status: Option<String>,
    pub target_timeframe: Option<String>,
    pub achieved_date: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAspirationRequest {
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub topic_id: Option<Uuid>,
    pub target_timeframe: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAspirationRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub topic_id: Option<Uuid>,
    pub status: Option<String>,
    pub target_timeframe: Option<String>,
    pub achieved_date: Option<chrono::DateTime<chrono::Utc>>,
}

// ============================================================================
// Task CRUD Operations
// ============================================================================

/// List all active tasks
pub async fn list_tasks(pool: &PgPool) -> Result<Vec<Task>> {
    let tasks = sqlx::query_as!(
        Task,
        r#"
        SELECT id, title, description, tags, topic_id, status,
               progress_percent, start_date, target_date, completed_date,
               is_active, created_at, updated_at
        FROM data.actions_task
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list tasks: {}", e)))?;

    Ok(tasks)
}

/// Get a specific task by ID
pub async fn get_task(pool: &PgPool, task_id: Uuid) -> Result<Task> {
    let task = sqlx::query_as!(
        Task,
        r#"
        SELECT id, title, description, tags, topic_id, status,
               progress_percent, start_date, target_date, completed_date,
               is_active, created_at, updated_at
        FROM data.actions_task
        WHERE id = $1 AND is_active = true
        "#,
        task_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get task: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Task not found: {}", task_id)))?;

    Ok(task)
}

/// Create a new task
pub async fn create_task(pool: &PgPool, req: CreateTaskRequest) -> Result<Task> {
    let task = sqlx::query_as!(
        Task,
        r#"
        INSERT INTO data.actions_task
            (title, description, tags, topic_id, start_date, target_date)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, title, description, tags, topic_id, status,
                  progress_percent, start_date, target_date, completed_date,
                  is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.tags.as_deref(),
        req.topic_id,
        req.start_date,
        req.target_date
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create task: {}", e)))?;

    Ok(task)
}

/// Update an existing task
pub async fn update_task(pool: &PgPool, task_id: Uuid, req: UpdateTaskRequest) -> Result<Task> {
    let task = sqlx::query_as!(
        Task,
        r#"
        UPDATE data.actions_task
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            tags = COALESCE($4, tags),
            topic_id = COALESCE($5, topic_id),
            status = COALESCE($6, status),
            progress_percent = COALESCE($7, progress_percent),
            start_date = COALESCE($8, start_date),
            target_date = COALESCE($9, target_date),
            completed_date = COALESCE($10, completed_date),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, tags, topic_id, status,
                  progress_percent, start_date, target_date, completed_date,
                  is_active, created_at, updated_at
        "#,
        task_id,
        req.title,
        req.description,
        req.tags.as_deref(),
        req.topic_id,
        req.status,
        req.progress_percent,
        req.start_date,
        req.target_date,
        req.completed_date
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update task: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Task not found: {}", task_id)))?;

    Ok(task)
}

/// Delete a task (soft delete - sets is_active = false)
pub async fn delete_task(pool: &PgPool, task_id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE data.actions_task
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        task_id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete task: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Task not found: {}", task_id)));
    }

    Ok(())
}

// ============================================================================
// Initiative CRUD Operations
// ============================================================================

pub async fn list_initiatives(pool: &PgPool) -> Result<Vec<Initiative>> {
    let items = sqlx::query_as!(
        Initiative,
        r#"
        SELECT id, title, description, tags, topic_id, status,
               progress_percent, start_date, target_date, completed_date,
               is_active, created_at, updated_at
        FROM data.actions_initiative
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list initiatives: {}", e)))?;

    Ok(items)
}

pub async fn get_initiative(pool: &PgPool, id: Uuid) -> Result<Initiative> {
    let item = sqlx::query_as!(
        Initiative,
        r#"
        SELECT id, title, description, tags, topic_id, status,
               progress_percent, start_date, target_date, completed_date,
               is_active, created_at, updated_at
        FROM data.actions_initiative
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get initiative: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Initiative not found: {}", id)))?;

    Ok(item)
}

pub async fn create_initiative(pool: &PgPool, req: CreateTaskRequest) -> Result<Initiative> {
    let item = sqlx::query_as!(
        Initiative,
        r#"
        INSERT INTO data.actions_initiative
            (title, description, tags, topic_id, start_date, target_date)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, title, description, tags, topic_id, status,
                  progress_percent, start_date, target_date, completed_date,
                  is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.tags.as_deref(),
        req.topic_id,
        req.start_date,
        req.target_date
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create initiative: {}", e)))?;

    Ok(item)
}

pub async fn update_initiative(pool: &PgPool, id: Uuid, req: UpdateTaskRequest) -> Result<Initiative> {
    let item = sqlx::query_as!(
        Initiative,
        r#"
        UPDATE data.actions_initiative
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            tags = COALESCE($4, tags),
            topic_id = COALESCE($5, topic_id),
            status = COALESCE($6, status),
            progress_percent = COALESCE($7, progress_percent),
            start_date = COALESCE($8, start_date),
            target_date = COALESCE($9, target_date),
            completed_date = COALESCE($10, completed_date),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, tags, topic_id, status,
                  progress_percent, start_date, target_date, completed_date,
                  is_active, created_at, updated_at
        "#,
        id,
        req.title,
        req.description,
        req.tags.as_deref(),
        req.topic_id,
        req.status,
        req.progress_percent,
        req.start_date,
        req.target_date,
        req.completed_date
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update initiative: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Initiative not found: {}", id)))?;

    Ok(item)
}

pub async fn delete_initiative(pool: &PgPool, id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE data.actions_initiative
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete initiative: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Initiative not found: {}", id)));
    }

    Ok(())
}

// ============================================================================
// Aspiration CRUD Operations
// ============================================================================

pub async fn list_aspirations(pool: &PgPool) -> Result<Vec<Aspiration>> {
    let items = sqlx::query_as!(
        Aspiration,
        r#"
        SELECT id, title, description, tags, topic_id, status,
               target_timeframe, achieved_date,
               is_active, created_at, updated_at
        FROM data.actions_aspiration
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list aspirations: {}", e)))?;

    Ok(items)
}

pub async fn get_aspiration(pool: &PgPool, id: Uuid) -> Result<Aspiration> {
    let item = sqlx::query_as!(
        Aspiration,
        r#"
        SELECT id, title, description, tags, topic_id, status,
               target_timeframe, achieved_date,
               is_active, created_at, updated_at
        FROM data.actions_aspiration
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get aspiration: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Aspiration not found: {}", id)))?;

    Ok(item)
}

pub async fn create_aspiration(pool: &PgPool, req: CreateAspirationRequest) -> Result<Aspiration> {
    let item = sqlx::query_as!(
        Aspiration,
        r#"
        INSERT INTO data.actions_aspiration
            (title, description, tags, topic_id, target_timeframe)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, title, description, tags, topic_id, status,
                  target_timeframe, achieved_date,
                  is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.tags.as_deref(),
        req.topic_id,
        req.target_timeframe
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create aspiration: {}", e)))?;

    Ok(item)
}

pub async fn update_aspiration(pool: &PgPool, id: Uuid, req: UpdateAspirationRequest) -> Result<Aspiration> {
    let item = sqlx::query_as!(
        Aspiration,
        r#"
        UPDATE data.actions_aspiration
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            tags = COALESCE($4, tags),
            topic_id = COALESCE($5, topic_id),
            status = COALESCE($6, status),
            target_timeframe = COALESCE($7, target_timeframe),
            achieved_date = COALESCE($8, achieved_date),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, tags, topic_id, status,
                  target_timeframe, achieved_date,
                  is_active, created_at, updated_at
        "#,
        id,
        req.title,
        req.description,
        req.tags.as_deref(),
        req.topic_id,
        req.status,
        req.target_timeframe,
        req.achieved_date
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update aspiration: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Aspiration not found: {}", id)))?;

    Ok(item)
}

pub async fn delete_aspiration(pool: &PgPool, id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE data.actions_aspiration
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete aspiration: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Aspiration not found: {}", id)));
    }

    Ok(())
}

/// List all distinct tags used across all active temporal pursuits (tasks, initiatives, aspirations)
pub async fn list_tags(pool: &PgPool) -> Result<Vec<String>> {
    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT tag
        FROM (
            SELECT unnest(tags) as tag FROM data.actions_task WHERE is_active = true AND tags IS NOT NULL
            UNION
            SELECT unnest(tags) as tag FROM data.actions_initiative WHERE is_active = true AND tags IS NOT NULL
            UNION
            SELECT unnest(tags) as tag FROM data.actions_aspiration WHERE is_active = true AND tags IS NOT NULL
        ) all_tags
        ORDER BY tag
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list tags: {}", e)))?;

    Ok(rows.into_iter().filter_map(|r| r.tag).collect())
}
