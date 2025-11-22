//! Ontologies API
//!
//! Endpoints for querying available ontology tables based on enabled streams.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashSet;

use crate::error::Result;
use crate::registry;

/// List available ontology tables based on enabled streams
///
/// This queries the database for enabled streams and maps them to ontology tables
/// using the source registry as the single source of truth.
/// Only returns tables that both (1) have enabled streams AND (2) actually exist in the database schema.
pub async fn list_available_ontologies(db: &PgPool) -> Result<Vec<String>> {
    // First, get all tables that actually exist in the data schema
    let existing_tables = sqlx::query!(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'data'
          AND table_name NOT LIKE 'stream_%'
          AND table_name NOT IN ('sources', 'streams', 'sync_logs')
          AND table_type = 'BASE TABLE'
        "#
    )
    .fetch_all(db)
    .await?;

    let existing_set: HashSet<String> = existing_tables
        .into_iter()
        .filter_map(|row| row.table_name)
        .collect();

    tracing::debug!(
        count = existing_set.len(),
        tables = ?existing_set,
        "Found existing ontology tables in database schema"
    );

    // Query enabled streams from database
    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT s.table_name
        FROM data.stream_connections s
        JOIN data.source_connections src ON s.source_connection_id = src.id
        WHERE s.is_enabled = true
          AND src.is_active = true
        "#
    )
    .fetch_all(db)
    .await?;

    // Map stream tables to ontology tables using source registry
    // AND filter by actual schema existence
    let mut ontologies = HashSet::new();
    for row in rows {
        if let Some((_source_name, stream)) = registry::get_stream_by_table_name(&row.table_name) {
            // Add all target ontology tables for this stream
            for target_table in &stream.target_ontologies {
                // Only include if table actually exists in the schema
                if existing_set.contains(*target_table) {
                    ontologies.insert(target_table.to_string());
                } else {
                    tracing::warn!(
                        stream = %row.table_name,
                        target = %target_table,
                        "Target ontology table does not exist in database schema (transform may not have run yet)"
                    );
                }
            }
        } else {
            tracing::debug!(
                table_name = %row.table_name,
                "No stream found in registry, skipping"
            );
        }
    }

    // Return sorted list
    let mut result: Vec<String> = ontologies.into_iter().collect();
    result.sort();

    tracing::info!(
        count = result.len(),
        tables = ?result,
        "Returning available ontology tables (enabled + existing)"
    );

    Ok(result)
}

/// Ontology overview information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyOverview {
    pub name: String,
    pub domain: String,
    pub record_count: i64,
    pub sample_record: Option<serde_json::Value>,
}

/// Get overview of all available ontologies with counts and sample records
pub async fn get_ontologies_overview(db: &PgPool) -> Result<Vec<OntologyOverview>> {
    let available_tables = list_available_ontologies(db).await?;
    let mut overviews = Vec::new();

    for table_name in available_tables {
        // Extract domain from table name (e.g., "health_heart_rate" -> "Health")
        let domain = extract_domain(&table_name);

        // Get record count
        let count_query = format!("SELECT COUNT(*) as count FROM data.{}", table_name);
        let count_result = sqlx::query_scalar::<_, i64>(&count_query)
            .fetch_one(db)
            .await;

        let record_count = match count_result {
            Ok(count) => count,
            Err(e) => {
                tracing::warn!(
                    table = %table_name,
                    error = %e,
                    "Failed to get record count for ontology table"
                );
                0
            }
        };

        // Get one random sample record if records exist
        let sample_record = if record_count > 0 {
            let sample_query = format!(
                "SELECT row_to_json(t) as record FROM (SELECT * FROM data.{} ORDER BY RANDOM() LIMIT 1) t",
                table_name
            );

            match sqlx::query_scalar::<_, serde_json::Value>(&sample_query)
                .fetch_one(db)
                .await
            {
                Ok(record) => Some(record),
                Err(e) => {
                    tracing::warn!(
                        table = %table_name,
                        error = %e,
                        "Failed to fetch sample record for ontology table"
                    );
                    None
                }
            }
        } else {
            None
        };

        overviews.push(OntologyOverview {
            name: table_name,
            domain,
            record_count,
            sample_record,
        });
    }

    // Sort by domain, then by name
    overviews.sort_by(|a, b| a.domain.cmp(&b.domain).then_with(|| a.name.cmp(&b.name)));

    Ok(overviews)
}

/// Extract domain name from table name
fn extract_domain(table_name: &str) -> String {
    let parts: Vec<&str> = table_name.split('_').collect();
    if parts.is_empty() {
        return "Unknown".to_string();
    }

    // Capitalize first letter
    let domain = parts[0];
    let mut chars = domain.chars();
    match chars.next() {
        None => "Unknown".to_string(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_streams() {
        // Verify that the main streams exist in the registry
        let known_streams = vec![
            "stream_google_gmail",
            "stream_google_calendar",
            "stream_notion_pages",
            "stream_ios_microphone",
            "stream_ariata_ai_chat",
        ];

        for stream_table in known_streams {
            let result = registry::get_stream_by_table_name(stream_table);
            assert!(
                result.is_some(),
                "Expected stream {} in registry, but not found",
                stream_table
            );

            // Verify target_ontologies is populated
            if let Some((_, stream)) = result {
                assert!(
                    !stream.target_ontologies.is_empty(),
                    "Expected target_ontologies for {}, but was empty",
                    stream_table
                );
            }
        }
    }
}
