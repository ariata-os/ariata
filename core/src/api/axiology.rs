//! Axiology API - Managing temporal pursuits and character development
//!
//! This module provides CRUD operations for:
//! - Tasks: Daily/weekly completable items
//! - Initiatives: Medium-term commitments (month-quarter)
//! - Aspirations: Long-term life goals (multi-year)
//! - Telos: Ultimate life purpose (singular active)
//! - Temperaments: Personality patterns and natural dispositions
//! - Virtues: Positive character patterns to cultivate
//! - Vices: Negative character patterns to resist
//! - Values: Foundational principles

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

// Simple axiology types (temperaments, virtues, vices, values)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Temperament {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub topic_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Virtue {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub topic_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vice {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub topic_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Value {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub topic_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSimpleRequest {
    pub title: String,
    pub description: Option<String>,
    pub topic_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSimpleRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub topic_id: Option<Uuid>,
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

// Telos type (singular active)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Telos {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub topic_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// Habit type (with frequency and streak tracking)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Habit {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub frequency: Option<String>,
    pub time_of_day: Option<String>,
    pub topic_id: Option<Uuid>,
    pub streak_count: Option<i32>,
    pub last_completed_date: Option<chrono::NaiveDate>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateHabitRequest {
    pub title: String,
    pub description: Option<String>,
    pub frequency: Option<String>,
    pub time_of_day: Option<String>,
    pub topic_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHabitRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub frequency: Option<String>,
    pub time_of_day: Option<String>,
    pub topic_id: Option<Uuid>,
    pub streak_count: Option<i32>,
    pub last_completed_date: Option<chrono::NaiveDate>,
}

// Preference type (with entity references)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Preference {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub preference_domain: Option<String>,
    pub valence: Option<String>,
    pub person_id: Option<Uuid>,
    pub place_id: Option<Uuid>,
    pub topic_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePreferenceRequest {
    pub title: String,
    pub description: Option<String>,
    pub preference_domain: Option<String>,
    pub valence: Option<String>,
    pub person_id: Option<Uuid>,
    pub place_id: Option<Uuid>,
    pub topic_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferenceRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub preference_domain: Option<String>,
    pub valence: Option<String>,
    pub person_id: Option<Uuid>,
    pub place_id: Option<Uuid>,
    pub topic_id: Option<Uuid>,
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
        FROM elt.axiology_task
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
        FROM elt.axiology_task
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
        INSERT INTO elt.axiology_task
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
        UPDATE elt.axiology_task
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
        UPDATE elt.axiology_task
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

/// List all distinct tags used across all active temporal pursuits (tasks, initiatives, aspirations)
pub async fn list_tags(pool: &PgPool) -> Result<Vec<String>> {
    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT tag
        FROM (
            SELECT unnest(tags) as tag FROM elt.axiology_task WHERE is_active = true AND tags IS NOT NULL
            UNION
            SELECT unnest(tags) as tag FROM elt.axiology_initiative WHERE is_active = true AND tags IS NOT NULL
            UNION
            SELECT unnest(tags) as tag FROM elt.axiology_aspiration WHERE is_active = true AND tags IS NOT NULL
        ) all_tags
        ORDER BY tag
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list tags: {}", e)))?;

    Ok(rows.into_iter().filter_map(|r| r.tag).collect())
}

// ============================================================================
// Temperament CRUD Operations
// ============================================================================

pub async fn list_temperaments(pool: &PgPool) -> Result<Vec<Temperament>> {
    let items = sqlx::query_as!(
        Temperament,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_temperament
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list temperaments: {}", e)))?;

    Ok(items)
}

pub async fn get_temperament(pool: &PgPool, id: Uuid) -> Result<Temperament> {
    let item = sqlx::query_as!(
        Temperament,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_temperament
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get temperament: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Temperament not found: {}", id)))?;

    Ok(item)
}

pub async fn create_temperament(pool: &PgPool, req: CreateSimpleRequest) -> Result<Temperament> {
    let item = sqlx::query_as!(
        Temperament,
        r#"
        INSERT INTO elt.axiology_temperament (title, description, topic_id)
        VALUES ($1, $2, $3)
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create temperament: {}", e)))?;

    Ok(item)
}

pub async fn update_temperament(pool: &PgPool, id: Uuid, req: UpdateSimpleRequest) -> Result<Temperament> {
    let item = sqlx::query_as!(
        Temperament,
        r#"
        UPDATE elt.axiology_temperament
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            topic_id = COALESCE($4, topic_id),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        id,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update temperament: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Temperament not found: {}", id)))?;

    Ok(item)
}

pub async fn delete_temperament(pool: &PgPool, id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE elt.axiology_temperament
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete temperament: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Temperament not found: {}", id)));
    }

    Ok(())
}

// ============================================================================
// Virtue CRUD Operations
// ============================================================================

pub async fn list_virtues(pool: &PgPool) -> Result<Vec<Virtue>> {
    let items = sqlx::query_as!(
        Virtue,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_virtue
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list virtues: {}", e)))?;

    Ok(items)
}

pub async fn get_virtue(pool: &PgPool, id: Uuid) -> Result<Virtue> {
    let item = sqlx::query_as!(
        Virtue,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_virtue
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get virtue: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Virtue not found: {}", id)))?;

    Ok(item)
}

pub async fn create_virtue(pool: &PgPool, req: CreateSimpleRequest) -> Result<Virtue> {
    let item = sqlx::query_as!(
        Virtue,
        r#"
        INSERT INTO elt.axiology_virtue (title, description, topic_id)
        VALUES ($1, $2, $3)
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create virtue: {}", e)))?;

    Ok(item)
}

pub async fn update_virtue(pool: &PgPool, id: Uuid, req: UpdateSimpleRequest) -> Result<Virtue> {
    let item = sqlx::query_as!(
        Virtue,
        r#"
        UPDATE elt.axiology_virtue
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            topic_id = COALESCE($4, topic_id),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        id,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update virtue: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Virtue not found: {}", id)))?;

    Ok(item)
}

pub async fn delete_virtue(pool: &PgPool, id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE elt.axiology_virtue
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete virtue: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Virtue not found: {}", id)));
    }

    Ok(())
}

// ============================================================================
// Vice CRUD Operations
// ============================================================================

pub async fn list_vices(pool: &PgPool) -> Result<Vec<Vice>> {
    let items = sqlx::query_as!(
        Vice,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_vice
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list vices: {}", e)))?;

    Ok(items)
}

pub async fn get_vice(pool: &PgPool, id: Uuid) -> Result<Vice> {
    let item = sqlx::query_as!(
        Vice,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_vice
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get vice: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Vice not found: {}", id)))?;

    Ok(item)
}

pub async fn create_vice(pool: &PgPool, req: CreateSimpleRequest) -> Result<Vice> {
    let item = sqlx::query_as!(
        Vice,
        r#"
        INSERT INTO elt.axiology_vice (title, description, topic_id)
        VALUES ($1, $2, $3)
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create vice: {}", e)))?;

    Ok(item)
}

pub async fn update_vice(pool: &PgPool, id: Uuid, req: UpdateSimpleRequest) -> Result<Vice> {
    let item = sqlx::query_as!(
        Vice,
        r#"
        UPDATE elt.axiology_vice
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            topic_id = COALESCE($4, topic_id),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        id,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update vice: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Vice not found: {}", id)))?;

    Ok(item)
}

pub async fn delete_vice(pool: &PgPool, id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE elt.axiology_vice
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete vice: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Vice not found: {}", id)));
    }

    Ok(())
}

// ============================================================================
// Value CRUD Operations
// ============================================================================

pub async fn list_values(pool: &PgPool) -> Result<Vec<Value>> {
    let items = sqlx::query_as!(
        Value,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_value
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list values: {}", e)))?;

    Ok(items)
}

pub async fn get_value(pool: &PgPool, id: Uuid) -> Result<Value> {
    let item = sqlx::query_as!(
        Value,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_value
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get value: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Value not found: {}", id)))?;

    Ok(item)
}

pub async fn create_value(pool: &PgPool, req: CreateSimpleRequest) -> Result<Value> {
    let item = sqlx::query_as!(
        Value,
        r#"
        INSERT INTO elt.axiology_value (title, description, topic_id)
        VALUES ($1, $2, $3)
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create value: {}", e)))?;

    Ok(item)
}

pub async fn update_value(pool: &PgPool, id: Uuid, req: UpdateSimpleRequest) -> Result<Value> {
    let item = sqlx::query_as!(
        Value,
        r#"
        UPDATE elt.axiology_value
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            topic_id = COALESCE($4, topic_id),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        id,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update value: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Value not found: {}", id)))?;

    Ok(item)
}

pub async fn delete_value(pool: &PgPool, id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE elt.axiology_value
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete value: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Value not found: {}", id)));
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
        FROM elt.axiology_initiative
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
        FROM elt.axiology_initiative
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
        INSERT INTO elt.axiology_initiative
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
        UPDATE elt.axiology_initiative
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
        UPDATE elt.axiology_initiative
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
        FROM elt.axiology_aspiration
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
        FROM elt.axiology_aspiration
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
        INSERT INTO elt.axiology_aspiration
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
        UPDATE elt.axiology_aspiration
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
        UPDATE elt.axiology_aspiration
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

// ============================================================================
// Telos CRUD Operations (with singular active constraint)
// ============================================================================

pub async fn list_telos(pool: &PgPool) -> Result<Vec<Telos>> {
    let items = sqlx::query_as!(
        Telos,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_telos
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list telos: {}", e)))?;

    Ok(items)
}

pub async fn get_telos(pool: &PgPool, id: Uuid) -> Result<Telos> {
    let item = sqlx::query_as!(
        Telos,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM elt.axiology_telos
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get telos: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Telos not found: {}", id)))?;

    Ok(item)
}

pub async fn create_telos(pool: &PgPool, req: CreateSimpleRequest) -> Result<Telos> {
    // Note: Database has unique index on is_active=true
    // So creating a new active telos will fail if one already exists
    let item = sqlx::query_as!(
        Telos,
        r#"
        INSERT INTO elt.axiology_telos (title, description, topic_id)
        VALUES ($1, $2, $3)
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create telos: {}", e)))?;

    Ok(item)
}

pub async fn update_telos(pool: &PgPool, id: Uuid, req: UpdateSimpleRequest) -> Result<Telos> {
    let item = sqlx::query_as!(
        Telos,
        r#"
        UPDATE elt.axiology_telos
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            topic_id = COALESCE($4, topic_id),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, topic_id, is_active, created_at, updated_at
        "#,
        id,
        req.title,
        req.description,
        req.topic_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update telos: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Telos not found: {}", id)))?;

    Ok(item)
}

pub async fn delete_telos(pool: &PgPool, id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE elt.axiology_telos
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete telos: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Telos not found: {}", id)));
    }

    Ok(())
}

// ============================================================================
// Habit CRUD Operations
// ============================================================================

pub async fn list_habits(pool: &PgPool) -> Result<Vec<Habit>> {
    let items = sqlx::query_as!(
        Habit,
        r#"
        SELECT id, title, description, frequency, time_of_day, topic_id,
               streak_count, last_completed_date, is_active, created_at, updated_at
        FROM elt.axiology_habit
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list habits: {}", e)))?;

    Ok(items)
}

pub async fn get_habit(pool: &PgPool, id: Uuid) -> Result<Habit> {
    let item = sqlx::query_as!(
        Habit,
        r#"
        SELECT id, title, description, frequency, time_of_day, topic_id,
               streak_count, last_completed_date, is_active, created_at, updated_at
        FROM elt.axiology_habit
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get habit: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Habit not found: {}", id)))?;

    Ok(item)
}

pub async fn create_habit(pool: &PgPool, req: CreateHabitRequest) -> Result<Habit> {
    let item = sqlx::query_as!(
        Habit,
        r#"
        INSERT INTO elt.axiology_habit
            (title, description, frequency, time_of_day, topic_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, title, description, frequency, time_of_day, topic_id,
                  streak_count, last_completed_date, is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.frequency,
        req.time_of_day,
        req.topic_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create habit: {}", e)))?;

    Ok(item)
}

pub async fn update_habit(pool: &PgPool, id: Uuid, req: UpdateHabitRequest) -> Result<Habit> {
    let item = sqlx::query_as!(
        Habit,
        r#"
        UPDATE elt.axiology_habit
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            frequency = COALESCE($4, frequency),
            time_of_day = COALESCE($5, time_of_day),
            topic_id = COALESCE($6, topic_id),
            streak_count = COALESCE($7, streak_count),
            last_completed_date = COALESCE($8, last_completed_date),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, frequency, time_of_day, topic_id,
                  streak_count, last_completed_date, is_active, created_at, updated_at
        "#,
        id,
        req.title,
        req.description,
        req.frequency,
        req.time_of_day,
        req.topic_id,
        req.streak_count,
        req.last_completed_date
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update habit: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Habit not found: {}", id)))?;

    Ok(item)
}

pub async fn delete_habit(pool: &PgPool, id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE elt.axiology_habit
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete habit: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Habit not found: {}", id)));
    }

    Ok(())
}

// ============================================================================
// Preference CRUD Operations
// ============================================================================

pub async fn list_preferences(pool: &PgPool) -> Result<Vec<Preference>> {
    let items = sqlx::query_as!(
        Preference,
        r#"
        SELECT id, title, description, preference_domain, valence,
               person_id, place_id, topic_id,
               is_active, created_at, updated_at
        FROM elt.axiology_preference
        WHERE is_active = true
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to list preferences: {}", e)))?;

    Ok(items)
}

pub async fn get_preference(pool: &PgPool, id: Uuid) -> Result<Preference> {
    let item = sqlx::query_as!(
        Preference,
        r#"
        SELECT id, title, description, preference_domain, valence,
               person_id, place_id, topic_id,
               is_active, created_at, updated_at
        FROM elt.axiology_preference
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to get preference: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Preference not found: {}", id)))?;

    Ok(item)
}

pub async fn create_preference(pool: &PgPool, req: CreatePreferenceRequest) -> Result<Preference> {
    let item = sqlx::query_as!(
        Preference,
        r#"
        INSERT INTO elt.axiology_preference
            (title, description, preference_domain, valence, person_id, place_id, topic_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, title, description, preference_domain, valence,
                  person_id, place_id, topic_id,
                  is_active, created_at, updated_at
        "#,
        req.title,
        req.description,
        req.preference_domain,
        req.valence,
        req.person_id,
        req.place_id,
        req.topic_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to create preference: {}", e)))?;

    Ok(item)
}

pub async fn update_preference(pool: &PgPool, id: Uuid, req: UpdatePreferenceRequest) -> Result<Preference> {
    let item = sqlx::query_as!(
        Preference,
        r#"
        UPDATE elt.axiology_preference
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            preference_domain = COALESCE($4, preference_domain),
            valence = COALESCE($5, valence),
            person_id = COALESCE($6, person_id),
            place_id = COALESCE($7, place_id),
            topic_id = COALESCE($8, topic_id),
            updated_at = NOW()
        WHERE id = $1 AND is_active = true
        RETURNING id, title, description, preference_domain, valence,
                  person_id, place_id, topic_id,
                  is_active, created_at, updated_at
        "#,
        id,
        req.title,
        req.description,
        req.preference_domain,
        req.valence,
        req.person_id,
        req.place_id,
        req.topic_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to update preference: {}", e)))?
    .ok_or_else(|| Error::NotFound(format!("Preference not found: {}", id)))?;

    Ok(item)
}

pub async fn delete_preference(pool: &PgPool, id: Uuid) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE elt.axiology_preference
        SET is_active = false, updated_at = NOW()
        WHERE id = $1 AND is_active = true
        "#,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("Failed to delete preference: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Preference not found: {}", id)));
    }

    Ok(())
}
