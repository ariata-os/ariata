use crate::database::Database;
use crate::timeline::boundaries::BoundaryCandidate;
use crate::timeline::events::BoundaryType;
use crate::Result;
use chrono::{DateTime, Duration, Utc};
use sqlx::Row;

/// Detect session boundaries from discrete event streams via temporal gap clustering
///
/// Algorithm: Group timestamped events into sessions based on inactivity gaps
/// - Events separated by <gap_threshold stay in same session
/// - Events separated by >gap_threshold start new session
///
/// Use cases:
/// - Voice transcriptions → conversation sessions (gap=2min)
/// - App usage → focused work sessions (gap=1min)
/// - Page visits → browsing sessions (gap=2min)
/// - Messages → chat conversations (gap=5min)
pub async fn detect(
    db: &Database,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    table: &str,
    timestamp_col: &str,
    duration_col: Option<&str>,
    gap_minutes: i64,
    min_duration_seconds: i64,
) -> Result<Vec<BoundaryCandidate>> {
    // Build query with optional duration column
    let duration_expr = duration_col.unwrap_or("0");

    let query = format!(
        "SELECT {} as event_time, {} as duration_sec
         FROM data.{}
         WHERE {} >= $1 AND {} <= $2
         ORDER BY {} ASC",
        timestamp_col, duration_expr, table, timestamp_col, timestamp_col, timestamp_col
    );

    let rows = sqlx::query(&query)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(db.pool())
        .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let mut boundaries = Vec::new();
    let mut session_start: Option<DateTime<Utc>> = None;
    let mut session_end: Option<DateTime<Utc>> = None;
    let mut event_count = 0;

    let gap_threshold = Duration::minutes(gap_minutes);
    let min_duration = Duration::seconds(min_duration_seconds);

    for (i, row) in rows.iter().enumerate() {
        let event_time: DateTime<Utc> = row.try_get("event_time")?;

        // Get duration (handle both i64 and f64 SQL types)
        let duration_sec: i64 = if let Ok(val) = row.try_get::<i64, _>("duration_sec") {
            val
        } else if let Ok(val) = row.try_get::<f64, _>("duration_sec") {
            val as i64
        } else if let Ok(val) = row.try_get::<Option<i64>, _>("duration_sec") {
            val.unwrap_or(0)
        } else if let Ok(val) = row.try_get::<Option<f64>, _>("duration_sec") {
            val.unwrap_or(0.0) as i64
        } else {
            0
        };

        let event_end = event_time + Duration::seconds(duration_sec);

        // First event - start first session
        if session_start.is_none() {
            session_start = Some(event_time);
            session_end = Some(event_end);
            event_count = 1;
            continue;
        }

        let last_end = session_end.unwrap();
        let gap = event_time - last_end;

        // Gap exceeds threshold - close current session and start new one
        if gap > gap_threshold {
            let session_duration = last_end - session_start.unwrap();

            if session_duration >= min_duration {
                // Emit boundaries for completed session
                boundaries.push(BoundaryCandidate {
                    timestamp: session_start.unwrap(),
                    boundary_type: BoundaryType::Begin,
                    source_ontology: String::new(), // Set by caller
                    fidelity: 0.0,                  // Set by caller
                    weight: 0,                      // Set by caller
                    metadata: serde_json::json!({
                        "type": "session_start",
                        "event_count": event_count,
                    }),
                });

                boundaries.push(BoundaryCandidate {
                    timestamp: last_end,
                    boundary_type: BoundaryType::End,
                    source_ontology: String::new(),
                    fidelity: 0.0,
                    weight: 0,                      // Set by caller
                    metadata: serde_json::json!({
                        "type": "session_end",
                        "event_count": event_count,
                    }),
                });
            }

            // Start new session
            session_start = Some(event_time);
            session_end = Some(event_end);
            event_count = 1;
        } else {
            // Extend current session
            session_end = Some(event_end);
            event_count += 1;
        }

        // Last event - close the session
        if i == rows.len() - 1 {
            let session_duration = session_end.unwrap() - session_start.unwrap();

            if session_duration >= min_duration {
                boundaries.push(BoundaryCandidate {
                    timestamp: session_start.unwrap(),
                    boundary_type: BoundaryType::Begin,
                    source_ontology: String::new(),
                    fidelity: 0.0,
                    weight: 0,                      // Set by caller
                    metadata: serde_json::json!({
                        "type": "session_start",
                        "event_count": event_count,
                    }),
                });

                boundaries.push(BoundaryCandidate {
                    timestamp: session_end.unwrap(),
                    boundary_type: BoundaryType::End,
                    source_ontology: String::new(),
                    fidelity: 0.0,
                    weight: 0,                      // Set by caller
                    metadata: serde_json::json!({
                        "type": "session_end",
                        "event_count": event_count,
                    }),
                });
            }
        }
    }

    tracing::debug!(
        "Discrete detector ({}): Processed {} events, {} boundaries ({} sessions)",
        table,
        rows.len(),
        boundaries.len(),
        boundaries.len() / 2
    );

    Ok(boundaries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gap_threshold_logic() {
        // Events at: 0min, 1min, 5min, 6min
        // With 2min gap threshold:
        // Session 1: 0-1min (2 events)
        // Session 2: 5-6min (2 events)

        let gap_threshold = Duration::minutes(2);

        let e1 = chrono::Utc::now();
        let e2 = e1 + Duration::minutes(1);
        let e3 = e1 + Duration::minutes(5);
        let e4 = e1 + Duration::minutes(6);

        // e1 -> e2: 1min gap < 2min threshold (same session)
        assert!((e2 - e1) < gap_threshold);

        // e2 -> e3: 4min gap > 2min threshold (new session)
        assert!((e3 - e2) > gap_threshold);

        // e3 -> e4: 1min gap < 2min threshold (same session)
        assert!((e4 - e3) < gap_threshold);
    }
}
