//! MCP Tool definitions for Ariata
//!
//! This module defines the tools exposed to AI assistants via the MCP protocol.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use sqlx::{Column, PgPool, Row, TypeInfo};
use crate::mcp::schema;

// ============================================================================
// Shared Utilities
// ============================================================================

/// Convert PostgreSQL rows to JSON array with comprehensive type support
///
/// This function handles NULL values, integers, floats, decimals, booleans,
/// dates, timestamps, JSON/JSONB, UUIDs, and falls back gracefully for unsupported types.
fn convert_rows_to_json(rows: &[sqlx::postgres::PgRow]) -> Vec<serde_json::Value> {
    let mut json_rows = Vec::new();

    for row in rows {
        let mut obj = serde_json::Map::new();

        for (i, col) in row.columns().iter().enumerate() {
            let col_name = col.name();

            // Try to extract value based on type, with comprehensive type support
            let value: serde_json::Value =
                // Try NULL first - check if the column value is NULL
                if let Ok(opt_val) = row.try_get::<Option<String>, _>(i) {
                    if opt_val.is_none() {
                        serde_json::Value::Null
                    } else {
                        // We know it's not NULL, so unwrap the Option
                        serde_json::Value::String(opt_val.unwrap())
                    }
                }
                // String types (if NULL check didn't work, try direct string)
                else if let Ok(v) = row.try_get::<String, _>(i) {
                    serde_json::Value::String(v)
                }
                // Integer types
                else if let Ok(v) = row.try_get::<i64, _>(i) {
                    serde_json::Value::Number(v.into())
                } else if let Ok(v) = row.try_get::<i32, _>(i) {
                    serde_json::Value::Number(v.into())
                } else if let Ok(v) = row.try_get::<i16, _>(i) {
                    serde_json::Value::Number(v.into())
                }
                // Float types
                else if let Ok(v) = row.try_get::<f64, _>(i) {
                    serde_json::json!(v)
                } else if let Ok(v) = row.try_get::<f32, _>(i) {
                    serde_json::json!(v)
                }
                // Decimal/Numeric types (PostgreSQL NUMERIC)
                else if let Ok(v) = row.try_get::<sqlx::types::Decimal, _>(i) {
                    // Convert Decimal to JSON number
                    // Try to parse as f64, fallback to string if precision is too high
                    v.to_string().parse::<f64>()
                        .map(|f| serde_json::json!(f))
                        .unwrap_or_else(|_| serde_json::Value::String(v.to_string()))
                }
                // Boolean
                else if let Ok(v) = row.try_get::<bool, _>(i) {
                    serde_json::Value::Bool(v)
                }
                // Date/Time types (convert to string)
                else if let Ok(v) = row.try_get::<chrono::NaiveDate, _>(i) {
                    serde_json::Value::String(v.format("%Y-%m-%d").to_string())
                } else if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(i) {
                    serde_json::Value::String(v.format("%Y-%m-%d %H:%M:%S").to_string())
                } else if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(i) {
                    serde_json::Value::String(v.to_rfc3339())
                } else if let Ok(v) = row.try_get::<chrono::DateTime<chrono::FixedOffset>, _>(i) {
                    serde_json::Value::String(v.to_rfc3339())
                } else if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Local>, _>(i) {
                    serde_json::Value::String(v.to_rfc3339())
                }
                // JSONB/JSON
                else if let Ok(v) = row.try_get::<serde_json::Value, _>(i) {
                    v
                }
                // UUID
                else if let Ok(v) = row.try_get::<uuid::Uuid, _>(i) {
                    serde_json::Value::String(v.to_string())
                }
                // Fallback: try to get raw value info for better error message
                else {
                    let type_info = col.type_info().name();
                    tracing::warn!(
                        "Column '{}' with PostgreSQL type '{}' could not be extracted - falling back to placeholder",
                        col_name,
                        type_info
                    );
                    serde_json::Value::String(format!("<unsupported type: {}>", type_info))
                };

            obj.insert(col_name.to_string(), value);
        }

        json_rows.push(serde_json::Value::Object(obj));
    }

    json_rows
}

// ============================================================================
// Query Ontology Tool
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryOntologyRequest {
    /// SQL query to execute (SELECT only, read-only)
    pub query: String,
    /// Optional limit on number of rows returned (default: 100, max: 1000)
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryOntologyResponse {
    /// Query results as JSON array
    pub rows: serde_json::Value,
    /// Number of rows returned
    pub row_count: usize,
    /// Whether more results exist beyond the returned rows
    pub has_more: bool,
    /// The limit that was applied to the query
    pub limit_applied: u32,
    /// Schema information for the queried tables
    pub schema_info: Option<String>,
}

/// Execute a read-only SQL query against ontology tables
pub async fn query_ontology(
    pool: &PgPool,
    request: QueryOntologyRequest,
) -> Result<QueryOntologyResponse, String> {
    // Security: Validate query length to prevent DOS
    if request.query.len() > 10000 {
        return Err("Query exceeds maximum length of 10KB".to_string());
    }

    // Security: Validate that query is read-only
    let query_lower = request.query.trim().to_lowercase();

    if !query_lower.starts_with("select") && !query_lower.starts_with("with") {
        return Err("Only SELECT queries are allowed".to_string());
    }

    // Check for dangerous keywords
    let dangerous_keywords = ["insert", "update", "delete", "drop", "create", "alter", "truncate"];
    for keyword in &dangerous_keywords {
        if query_lower.contains(keyword) {
            return Err(format!("Query contains forbidden keyword: {}", keyword));
        }
    }

    // Apply limit (fetch one extra row to detect if more results exist)
    let limit = request.limit.unwrap_or(100).min(1000);
    let fetch_limit = limit + 1; // Fetch one extra to detect has_more

    let query_with_limit = if query_lower.contains("limit") {
        // User provided their own LIMIT, use as-is
        request.query.clone()
    } else {
        format!("{} LIMIT {}", request.query, fetch_limit)
    };

    // Execute query in a read-only transaction
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Set transaction to read-only
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    // Execute the query
    let rows = sqlx::query(&query_with_limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("Query execution failed: {}", e))?;

    // Convert rows to JSON using shared utility function
    let json_rows = convert_rows_to_json(&rows);

    // Check if there are more results
    let has_more = json_rows.len() > limit as usize;
    let final_rows: Vec<_> = json_rows.into_iter().take(limit as usize).collect();

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(QueryOntologyResponse {
        row_count: final_rows.len(),
        rows: serde_json::Value::Array(final_rows),
        has_more,
        limit_applied: limit,
        schema_info: None,
    })
}

// ============================================================================
// Query Axiology Tool
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryAxiologyRequest {
    /// SQL query to execute (SELECT only, read-only)
    pub query: String,
    /// Optional limit on number of rows returned (default: 100, max: 1000)
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryAxiologyResponse {
    /// Query results as JSON array
    pub rows: serde_json::Value,
    /// Number of rows returned
    pub row_count: usize,
    /// Whether more results exist beyond the returned rows
    pub has_more: bool,
    /// The limit that was applied to the query
    pub limit_applied: u32,
    /// Schema information for the queried tables
    pub schema_info: Option<String>,
}

/// Execute a read-only SQL query against axiology tables
/// (values, telos, goals, virtues, vices, habits, temperaments, preferences)
pub async fn query_axiology(
    pool: &PgPool,
    request: QueryAxiologyRequest,
) -> Result<QueryAxiologyResponse, String> {
    // Security: Validate query length to prevent DOS
    if request.query.len() > 10000 {
        return Err("Query exceeds maximum length of 10KB".to_string());
    }

    // Security: Validate that query is read-only
    let query_lower = request.query.trim().to_lowercase();

    if !query_lower.starts_with("select") && !query_lower.starts_with("with") {
        return Err("Only SELECT queries are allowed".to_string());
    }

    // Check for dangerous keywords
    let dangerous_keywords = ["insert", "update", "delete", "drop", "create", "alter", "truncate"];
    for keyword in &dangerous_keywords {
        if query_lower.contains(keyword) {
            return Err(format!("Query contains forbidden keyword: {}", keyword));
        }
    }

    // Apply limit (fetch one extra row to detect if more results exist)
    let limit = request.limit.unwrap_or(100).min(1000);
    let fetch_limit = limit + 1; // Fetch one extra to detect has_more

    let query_with_limit = if query_lower.contains("limit") {
        // User provided their own LIMIT, use as-is
        request.query.clone()
    } else {
        format!("{} LIMIT {}", request.query, fetch_limit)
    };

    // Execute query in a read-only transaction
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // Set transaction to read-only
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    // Execute the query
    let rows = sqlx::query(&query_with_limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("Query execution failed: {}", e))?;

    // Convert rows to JSON using shared utility function
    let json_rows = convert_rows_to_json(&rows);

    // Check if there are more results
    let has_more = json_rows.len() > limit as usize;
    let final_rows: Vec<_> = json_rows.into_iter().take(limit as usize).collect();

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(QueryAxiologyResponse {
        row_count: final_rows.len(),
        rows: serde_json::Value::Array(final_rows),
        has_more,
        limit_applied: limit,
        schema_info: None,
    })
}

// ============================================================================
// List Sources Tool
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSourcesRequest {
    // Empty struct - this tool takes no parameters
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSourcesResponse {
    pub sources: Vec<SourceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceInfo {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub status: String,
    pub enabled_streams: i32,
    pub created_at: String,
    pub last_sync: Option<String>,
}

/// List all data sources and their status
pub async fn list_sources(pool: &PgPool) -> Result<ListSourcesResponse, String> {
    let rows = sqlx::query(
        r#"
        SELECT
            s.id::text as id,
            s.name,
            s.provider as source_type,
            CASE
                WHEN NOT s.is_active THEN 'inactive'
                WHEN s.pairing_status = 'pending' THEN 'pending'
                WHEN s.pairing_status = 'revoked' THEN 'revoked'
                WHEN s.error_message IS NOT NULL THEN 'error'
                ELSE 'active'
            END as status,
            COUNT(DISTINCT st.stream_name) FILTER (WHERE st.is_enabled = true)::int as enabled_streams,
            s.created_at::text as created_at,
            MAX(j.completed_at)::text as last_sync
        FROM sources s
        LEFT JOIN streams st ON st.source_id = s.id
        LEFT JOIN jobs j ON j.source_id = s.id AND j.status = 'completed'
        GROUP BY s.id, s.name, s.provider, s.is_active, s.pairing_status, s.error_message, s.created_at
        ORDER BY s.created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut sources = Vec::new();
    for row in rows {
        sources.push(SourceInfo {
            id: row.get("id"),
            name: row.get("name"),
            source_type: row.get("source_type"),
            status: row.get("status"),
            enabled_streams: row.get("enabled_streams"),
            created_at: row.get("created_at"),
            last_sync: row.get("last_sync"),
        });
    }

    Ok(ListSourcesResponse { sources })
}

// ============================================================================
// List Ontology Tables Tool
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListOntologyTablesRequest {
    // Empty struct - this tool takes no parameters
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListOntologyTablesResponse {
    pub tables: Vec<OntologyTableInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OntologyTableInfo {
    pub table_name: String,
    pub columns: Vec<String>,
    pub row_count: Option<i64>,
}

/// List all available ontology tables with their schemas
pub async fn list_ontology_tables(pool: &PgPool) -> Result<ListOntologyTablesResponse, String> {
    let table_names = schema::list_ontology_tables(pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut tables = Vec::new();

    for table_name in table_names {
        // Get schema
        let table_schema = schema::get_table_schema(pool, &table_name)
            .await
            .map_err(|e| e.to_string())?;

        // Get row count
        let row_count: Option<i64> = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM elt.{}",
            table_name
        ))
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        tables.push(OntologyTableInfo {
            table_name,
            columns: table_schema.columns.iter().map(|c| c.name.clone()).collect(),
            row_count,
        });
    }

    Ok(ListOntologyTablesResponse { tables })
}

// ============================================================================
// Get Table Schema Tool
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTableSchemaRequest {
    pub table_name: String,
}

/// Get detailed schema for a specific ontology table
pub async fn get_table_schema(
    pool: &PgPool,
    request: GetTableSchemaRequest,
) -> Result<schema::TableSchema, String> {
    schema::get_table_schema(pool, &request.table_name)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Trigger Sync Tool
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TriggerSyncRequest {
    pub source_id: String,
    pub stream_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TriggerSyncResponse {
    pub job_ids: Vec<String>,
    pub message: String,
}

/// Trigger a manual sync for a source or specific stream
pub async fn trigger_sync(
    pool: &PgPool,
    request: TriggerSyncRequest,
) -> Result<TriggerSyncResponse, String> {
    // Parse source UUID
    let source_uuid = uuid::Uuid::parse_str(&request.source_id)
        .map_err(|_| "Invalid source ID format".to_string())?;

    // Get streams to sync
    let streams: Vec<(String,)> = if let Some(stream_name) = &request.stream_name {
        sqlx::query_as(
            "SELECT name FROM elt.streams WHERE source_id = $1 AND name = $2 AND enabled = true"
        )
        .bind(source_uuid)
        .bind(stream_name)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
    } else {
        sqlx::query_as(
            "SELECT name FROM elt.streams WHERE source_id = $1 AND enabled = true"
        )
        .bind(source_uuid)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?
    };

    if streams.is_empty() {
        return Err("No enabled streams found for this source".to_string());
    }

    // Create sync jobs
    let mut job_ids = Vec::new();

    for (stream_name,) in streams {
        let job_id = uuid::Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO elt.jobs (id, source_id, stream_name, job_type, status, created_at)
            VALUES ($1, $2, $3, 'sync', 'pending', NOW())
            "#
        )
        .bind(job_id)
        .bind(source_uuid)
        .bind(&stream_name)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

        job_ids.push(job_id.to_string());
    }

    let message = format!(
        "Created {} sync job(s) for source {}",
        job_ids.len(),
        request.source_id
    );

    Ok(TriggerSyncResponse { job_ids, message })
}

// ============================================================================
// Query Narratives Tool
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryNarrativesRequest {
    /// Date to query narratives for (YYYY-MM-DD format)
    pub date: String,
    /// Optional location filter (e.g., "Rome", "San Francisco")
    #[serde(default)]
    pub location: Option<String>,
    /// Optional person filter (name of person met/interacted with)
    #[serde(default)]
    pub person: Option<String>,
    /// Optional narrative type filter (action, event, day, week, chapter, telos)
    #[serde(default)]
    pub narrative_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryNarrativesResponse {
    /// Narrative summaries as JSON array
    pub narratives: serde_json::Value,
    /// Number of narratives returned
    pub narrative_count: usize,
}

/// Query narrative biography summaries for a specific date, location, or person
pub async fn query_narratives(
    pool: &PgPool,
    request: QueryNarrativesRequest,
) -> Result<QueryNarrativesResponse, String> {
    // Parse the date
    let date = chrono::NaiveDate::parse_from_str(&request.date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format (expected YYYY-MM-DD): {}", e))?;

    let day_start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let day_end = (date + chrono::Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    // Build the query dynamically based on filters
    let mut query = String::from(
        "SELECT narrative_text, narrative_type, time_start, time_end, confidence_score \
         FROM elt.narrative_chunks \
         WHERE time_start >= $1 AND time_end <= $2"
    );

    // Add narrative_type filter if provided
    if let Some(ref nt) = request.narrative_type {
        query.push_str(&format!(" AND narrative_type = '{}'", nt));
    } else {
        // Default to day and event narratives
        query.push_str(" AND narrative_type IN ('day', 'event')");
    }

    query.push_str(" ORDER BY time_start DESC LIMIT 10");

    // Execute the query
    let rows = sqlx::query(&query)
        .bind(day_start)
        .bind(day_end)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Query execution failed: {}", e))?;

    // Convert rows to JSON
    let json_rows = convert_rows_to_json(&rows);

    Ok(QueryNarrativesResponse {
        narrative_count: json_rows.len(),
        narratives: serde_json::Value::Array(json_rows),
    })
}
