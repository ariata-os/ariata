use crate::database::Database;
use crate::timeline::boundaries::BoundaryCandidate;
use crate::timeline::events::BoundaryType;
use crate::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

/// Direct extraction of boundaries from pre-defined interval data
///
/// Algorithm: Extract existing start_time/end_time pairs from database
/// - No transformation or clustering needed
/// - Optionally apply SQL filters (e.g., minimum duration)
///
/// Use cases:
/// - Calendar events (explicit begin/end times)
/// - Sleep sessions (HealthKit analysis)
/// - Location visits (GPS clustering already done)
/// - Meeting blocks (scheduled intervals)
pub async fn detect(
    db: &Database,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    table: &str,
    start_col: &str,
    end_col: &str,
    filters: &[(&str, &str)],
) -> Result<Vec<BoundaryCandidate>> {
    // Build WHERE clause with time range and optional filters
    let mut where_clauses = vec![
        format!("{} < $2", start_col),  // Interval starts before window end
        format!("{} > $1", end_col),    // Interval ends after window start
    ];

    for (field, condition) in filters {
        where_clauses.push(format!("{} {}", field, condition));
    }

    let where_clause = where_clauses.join(" AND ");

    // Query for intervals overlapping the time window
    let query = format!(
        "SELECT {} as start_time, {} as end_time
         FROM data.{}
         WHERE {}
         ORDER BY {} ASC",
        start_col, end_col, table, where_clause, start_col
    );

    let rows = sqlx::query(&query)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(db.pool())
        .await?;

    let mut boundaries = Vec::new();

    for row in &rows {
        let interval_start: DateTime<Utc> = row.try_get("start_time")?;
        let interval_end: DateTime<Utc> = row.try_get("end_time")?;

        // Begin boundary
        boundaries.push(BoundaryCandidate {
            timestamp: interval_start,
            boundary_type: BoundaryType::Begin,
            source_ontology: String::new(), // Set by caller
            fidelity: 0.0,                  // Set by caller
            weight: 0,                      // Set by caller
            metadata: serde_json::json!({
                "type": "interval_start",
            }),
        });

        // End boundary
        boundaries.push(BoundaryCandidate {
            timestamp: interval_end,
            boundary_type: BoundaryType::End,
            source_ontology: String::new(),
            fidelity: 0.0,
            weight: 0,                      // Set by caller
            metadata: serde_json::json!({
                "type": "interval_end",
            }),
        });
    }

    tracing::debug!(
        "Interval detector ({}): Found {} intervals, {} boundaries",
        table,
        rows.len(),
        boundaries.len()
    );

    Ok(boundaries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_where_clause_construction() {
        // Test that filters are properly formatted
        let filters = vec![
            ("duration", "> INTERVAL '30 minutes'"),
            ("is_active", "= true"),
        ];

        let mut where_clauses = vec![
            "start_time < $2".to_string(),
            "end_time > $1".to_string(),
        ];

        for (field, condition) in &filters {
            where_clauses.push(format!("{} {}", field, condition));
        }

        let where_clause = where_clauses.join(" AND ");

        assert!(where_clause.contains("start_time < $2"));
        assert!(where_clause.contains("end_time > $1"));
        assert!(where_clause.contains("duration > INTERVAL '30 minutes'"));
        assert!(where_clause.contains("is_active = true"));
    }

    #[test]
    fn test_time_window_overlap_logic() {
        // Window: 10:00 - 11:00
        // Interval 1:  9:00 - 10:30 (overlaps - starts before, ends during)
        // Interval 2: 10:15 - 10:45 (overlaps - fully contained)
        // Interval 3: 10:45 - 11:30 (overlaps - starts during, ends after)
        // Interval 4:  8:00 -  9:00 (no overlap - ends before)
        // Interval 5: 12:00 - 13:00 (no overlap - starts after)

        let window_start = chrono::Utc::now();
        let window_end = window_start + chrono::Duration::hours(1);

        let interval_1_start = window_start - chrono::Duration::hours(1);
        let interval_1_end = window_start + chrono::Duration::minutes(30);
        assert!(interval_1_start < window_end && interval_1_end > window_start);

        let interval_2_start = window_start + chrono::Duration::minutes(15);
        let interval_2_end = window_start + chrono::Duration::minutes(45);
        assert!(interval_2_start < window_end && interval_2_end > window_start);

        let interval_3_start = window_start + chrono::Duration::minutes(45);
        let interval_3_end = window_end + chrono::Duration::minutes(30);
        assert!(interval_3_start < window_end && interval_3_end > window_start);

        let interval_4_start = window_start - chrono::Duration::hours(2);
        let interval_4_end = window_start - chrono::Duration::hours(1);
        assert!(!(interval_4_start < window_end && interval_4_end > window_start));

        let interval_5_start = window_end + chrono::Duration::hours(1);
        let interval_5_end = window_end + chrono::Duration::hours(2);
        assert!(!(interval_5_start < window_end && interval_5_end > window_start));
    }
}
