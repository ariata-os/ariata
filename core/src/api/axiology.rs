//! Axiology API - Managing values and character development
//!
//! This module provides CRUD operations for:
//! - Values: Foundational principles
//! - Telos: Ultimate life purpose (singular active)
//! - Temperaments: Personality patterns and natural dispositions
//! - Virtues: Positive character patterns to cultivate
//! - Vices: Negative character patterns to resist
//! - Habits: Daily practices
//! - Preferences: Affinities with entities
//!
//! These represent the "being" layer - your value system and character.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Error, Result};

// ============================================================================
// Types
// ============================================================================

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
// Temperament CRUD Operations
// ============================================================================

pub async fn list_temperaments(pool: &PgPool) -> Result<Vec<Temperament>> {
    let items = sqlx::query_as!(
        Temperament,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM data.axiology_temperament
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
        FROM data.axiology_temperament
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
        INSERT INTO data.axiology_temperament (title, description, topic_id)
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
        UPDATE data.axiology_temperament
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
        UPDATE data.axiology_temperament
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
        FROM data.axiology_virtue
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
        FROM data.axiology_virtue
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
        INSERT INTO data.axiology_virtue (title, description, topic_id)
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
        UPDATE data.axiology_virtue
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
        UPDATE data.axiology_virtue
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
        FROM data.axiology_vice
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
        FROM data.axiology_vice
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
        INSERT INTO data.axiology_vice (title, description, topic_id)
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
        UPDATE data.axiology_vice
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
        UPDATE data.axiology_vice
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
        FROM data.axiology_value
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
        FROM data.axiology_value
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
        INSERT INTO data.axiology_value (title, description, topic_id)
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
        UPDATE data.axiology_value
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
        UPDATE data.axiology_value
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
// Telos CRUD Operations (with singular active constraint)
// ============================================================================

pub async fn list_telos(pool: &PgPool) -> Result<Vec<Telos>> {
    let items = sqlx::query_as!(
        Telos,
        r#"
        SELECT id, title, description, topic_id, is_active, created_at, updated_at
        FROM data.axiology_telos
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
        FROM data.axiology_telos
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
        INSERT INTO data.axiology_telos (title, description, topic_id)
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
        UPDATE data.axiology_telos
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
        UPDATE data.axiology_telos
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
        FROM data.axiology_habit
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
        FROM data.axiology_habit
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
        INSERT INTO data.axiology_habit
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
        UPDATE data.axiology_habit
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
        UPDATE data.axiology_habit
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
        FROM data.axiology_preference
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
        FROM data.axiology_preference
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
        INSERT INTO data.axiology_preference
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
        UPDATE data.axiology_preference
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
        UPDATE data.axiology_preference
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
