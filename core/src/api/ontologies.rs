//! Ontologies API
//!
//! Endpoints for querying available ontology tables based on enabled streams.

use sqlx::PgPool;
use std::collections::HashSet;

use crate::error::Result;
use crate::transforms;

/// List available ontology tables based on enabled streams
///
/// This queries the database for enabled streams and maps them to ontology tables
/// using the transform registry as the single source of truth.
pub async fn list_available_ontologies(db: &PgPool) -> Result<Vec<String>> {
    // Query enabled streams from database
    let rows = sqlx::query!(
        r#"
        SELECT DISTINCT s.table_name
        FROM elt.streams s
        JOIN elt.sources src ON s.source_id = src.id
        WHERE s.is_enabled = true
          AND src.is_active = true
        "#
    )
    .fetch_all(db)
    .await?;

    // Map stream tables to ontology tables using transform registry
    let mut ontologies = HashSet::new();
    for row in rows {
        if let Ok(route) = transforms::get_transform_route(&row.table_name) {
            // Add all target ontology tables for this stream
            for target_table in route.target_tables {
                ontologies.insert(target_table.to_string());
            }
        } else {
            tracing::debug!(
                table_name = %row.table_name,
                "No transform route found for stream, skipping"
            );
        }
    }

    // Return sorted list
    let mut result: Vec<String> = ontologies.into_iter().collect();
    result.sort();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transforms_registry_has_routes() {
        // Verify that the main streams have transform routes
        let known_streams = vec![
            "stream_google_gmail",
            "stream_google_calendar",
            "stream_notion_pages",
            "stream_ios_microphone",
            "stream_ariata_ai_chat",
        ];

        for stream in known_streams {
            let route = transforms::get_transform_route(stream);
            assert!(
                route.is_ok(),
                "Expected transform route for {}, but got error: {:?}",
                stream,
                route.err()
            );
        }
    }
}
