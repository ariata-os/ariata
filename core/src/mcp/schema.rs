//! Dynamic schema introspection for ontology tables
//!
//! This module provides utilities to dynamically discover and describe
//! the schema of ontology tables in the `data` schema.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

/// Information about a database column
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub column_default: Option<String>,
}

/// Schema information for an ontology table
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableSchema {
    pub table_name: String,
    pub columns: Vec<ColumnInfo>,
    pub description: Option<String>,
}

/// Get the schema for a specific table in the data schema
pub async fn get_table_schema(pool: &PgPool, table_name: &str) -> Result<TableSchema, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            column_name,
            data_type,
            is_nullable,
            column_default
        FROM information_schema.columns
        WHERE table_schema = 'data'
        AND table_name = $1
        ORDER BY ordinal_position
        "#,
    )
    .bind(table_name)
    .fetch_all(pool)
    .await?;

    let mut columns = Vec::new();
    for row in rows {
        columns.push(ColumnInfo {
            name: row.get("column_name"),
            data_type: row.get("data_type"),
            is_nullable: row.get::<String, _>("is_nullable") == "YES",
            column_default: row.get("column_default"),
        });
    }

    Ok(TableSchema {
        table_name: table_name.to_string(),
        columns,
        description: None,
    })
}

/// List all ontology tables in the data schema
pub async fn list_ontology_tables(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let tables: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'data'
        AND table_name NOT IN ('sources', 'streams', 'jobs', 'devices', 'pending_device_pairings')
        ORDER BY table_name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(tables.into_iter().map(|(name,)| name).collect())
}
