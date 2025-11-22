//! Substance Extraction
//!
//! Extracts the 6 W's (who/where/what/how/why) from ontology primitives
//! for a given event segment.
//!
//! ## Extraction Logic
//!
//! - WHERE: Query location_visit for place_id + label
//! - WHO: Query praxis_calendar for participant_ids + names
//! - WHAT: Query activity_app_usage for primary category + apps
//! - HOW: Query health primitives for biometric context (future)
//! - WHY: Extract declared_intent from calendar event titles

use uuid::Uuid;
use serde_json::{json, Value as JsonValue};

use crate::database::Database;
use crate::error::Result;
use super::segmentation::EventSegment;

/// Extracted substance data for a narrative primitive
#[derive(Debug, Clone)]
pub struct SubstanceData {
    // WHERE
    pub place_id: Option<Uuid>,
    pub place_label: Option<String>,
    pub is_transit: bool,

    // WHO
    pub participant_ids: Vec<Uuid>,
    pub participant_context: JsonValue,

    // WHAT
    pub primary_activity: Option<String>,
    pub secondary_activities: Vec<String>,
    pub activity_payload: JsonValue,

    // HOW
    pub biometric_context: JsonValue,

    // WHY
    pub declared_intent: Option<String>,
}

impl Default for SubstanceData {
    fn default() -> Self {
        Self {
            place_id: None,
            place_label: None,
            is_transit: false,
            participant_ids: vec![],
            participant_context: json!([]),
            primary_activity: None,
            secondary_activities: vec![],
            activity_payload: json!({}),
            biometric_context: json!({}),
            declared_intent: None,
        }
    }
}

/// Extract substance (who/where/what/how/why) for an event segment
pub async fn extract_substance(db: &Database, segment: &EventSegment) -> Result<SubstanceData> {
    let mut substance = SubstanceData::default();

    // Extract WHERE (place)
    if let Some(place_data) = extract_place(db, segment).await? {
        substance.place_id = place_data.0;
        substance.place_label = place_data.1;
    }

    // Extract WHO (participants from calendar)
    if let Some(participant_data) = extract_participants(db, segment).await? {
        substance.participant_ids = participant_data.0;
        substance.participant_context = participant_data.1;
    }

    // Extract WHAT (activity from app usage)
    if let Some(activity_data) = extract_activity(db, segment).await? {
        substance.primary_activity = Some(activity_data.0);
        substance.secondary_activities = activity_data.1;
        substance.activity_payload = activity_data.2;
    }

    // Extract WHY (declared intent from calendar)
    if let Some(intent) = extract_intent(db, segment).await? {
        substance.declared_intent = Some(intent);
    }

    // HOW (biometric context) - placeholder for now
    // Future: Extract heart rate, HRV, ambient noise, etc.

    Ok(substance)
}

/// Extract place information (WHERE)
async fn extract_place(
    db: &Database,
    segment: &EventSegment,
) -> Result<Option<(Option<Uuid>, Option<String>)>> {
    // Query location_visit overlapping this segment
    let row = sqlx::query!(
        r#"
        SELECT
            lv.id,
            lv.place_id,
            ep.canonical_name as place_name
        FROM data.location_visit lv
        LEFT JOIN data.entities_place ep ON lv.place_id = ep.id
        WHERE lv.start_time <= $2
          AND lv.end_time >= $1
        ORDER BY (lv.end_time - lv.start_time) DESC
        LIMIT 1
        "#,
        segment.start_time,
        segment.end_time
    )
    .fetch_optional(db.pool())
    .await?;

    if let Some(row) = row {
        Ok(Some((row.place_id, Some(row.place_name))))
    } else {
        Ok(None)
    }
}

/// Extract participants (WHO)
async fn extract_participants(
    db: &Database,
    segment: &EventSegment,
) -> Result<Option<(Vec<Uuid>, JsonValue)>> {
    // Query calendar events overlapping this segment
    let rows = sqlx::query!(
        r#"
        SELECT
            attendee_person_ids,
            title
        FROM data.praxis_calendar
        WHERE start_time <= $2
          AND end_time >= $1
          AND array_length(attendee_person_ids, 1) > 0
        "#,
        segment.start_time,
        segment.end_time
    )
    .fetch_all(db.pool())
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut participant_ids = Vec::new();
    let mut participant_context = Vec::new();

    for row in rows {
        if let Some(person_ids) = row.attendee_person_ids {
            for person_id in person_ids {
                // Fetch person details
                if let Ok(Some(person_row)) = sqlx::query!(
                    r#"
                    SELECT id, canonical_name
                    FROM data.entities_person
                    WHERE id = $1
                    "#,
                    person_id
                )
                .fetch_optional(db.pool())
                .await
                {
                    participant_ids.push(person_row.id);
                    participant_context.push(json!({
                        "name": person_row.canonical_name,
                        "role": "participant"
                    }));
                }
            }
        }
    }

    if participant_ids.is_empty() {
        Ok(None)
    } else {
        Ok(Some((participant_ids, json!(participant_context))))
    }
}

/// Extract activity (WHAT)
async fn extract_activity(
    db: &Database,
    segment: &EventSegment,
) -> Result<Option<(String, Vec<String>, JsonValue)>> {
    // Query app usage overlapping this segment
    let rows = sqlx::query!(
        r#"
        SELECT
            app_name,
            app_category,
            window_title,
            start_time,
            end_time
        FROM data.activity_app_usage
        WHERE start_time <= $2
          AND end_time >= $1
        ORDER BY (end_time - start_time) DESC
        "#,
        segment.start_time,
        segment.end_time
    )
    .fetch_all(db.pool())
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    // Calculate dominant category by total duration
    let mut category_durations: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut app_usage_details = Vec::new();

    for row in &rows {
        let duration = (row.end_time - row.start_time).num_seconds();
        let category = row.app_category.clone().unwrap_or_else(|| "Unknown".to_string());

        *category_durations.entry(category.clone()).or_insert(0) += duration;

        app_usage_details.push(json!({
            "app_name": row.app_name,
            "category": category,
            "window_title": row.window_title,
            "duration_seconds": duration
        }));
    }

    // Find dominant category
    let primary_category = category_durations
        .iter()
        .max_by_key(|(_, &duration)| duration)
        .map(|(cat, _)| categorize_activity(cat))
        .unwrap_or_else(|| "Unknown".to_string());

    // Secondary categories (others with >10% of total time)
    let total_duration: i64 = category_durations.values().sum();
    let secondary_categories: Vec<String> = category_durations
        .iter()
        .filter(|(cat, &duration)| {
            let mapped_cat = categorize_activity(cat);
            mapped_cat != primary_category && duration * 10 > total_duration
        })
        .map(|(cat, _)| categorize_activity(cat))
        .collect();

    // Build activity payload
    let activity_payload = json!({
        "apps": app_usage_details,
        "dominant_category_duration": category_durations.get(&primary_category).unwrap_or(&0)
    });

    Ok(Some((primary_category, secondary_categories, activity_payload)))
}

/// Map app category to high-level activity
fn categorize_activity(app_category: &str) -> String {
    match app_category.to_lowercase().as_str() {
        "productivity" | "developer-tools" | "utilities" => "Deep Work".to_string(),
        "social-networking" | "social" => "Socializing".to_string(),
        "entertainment" | "video" | "games" => "Leisure".to_string(),
        "communication" | "email" => "Communication".to_string(),
        "health-and-fitness" | "exercise" => "Exercise".to_string(),
        "navigation" | "travel" | "maps" => "Commute".to_string(),
        "food-and-drink" | "lifestyle" => "Dining".to_string(),
        "education" | "books" | "reference" => "Learning".to_string(),
        _ => "Unknown".to_string(),
    }
}

/// Extract declared intent (WHY)
async fn extract_intent(db: &Database, segment: &EventSegment) -> Result<Option<String>> {
    // Query calendar events overlapping this segment
    let row = sqlx::query!(
        r#"
        SELECT title
        FROM data.praxis_calendar
        WHERE start_time <= $2
          AND end_time >= $1
        ORDER BY (end_time - start_time) DESC
        LIMIT 1
        "#,
        segment.start_time,
        segment.end_time
    )
    .fetch_optional(db.pool())
    .await?;

    Ok(row.and_then(|r| r.title))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_activity() {
        assert_eq!(categorize_activity("productivity"), "Deep Work");
        assert_eq!(categorize_activity("social-networking"), "Socializing");
        assert_eq!(categorize_activity("entertainment"), "Leisure");
        assert_eq!(categorize_activity("navigation"), "Commute");
        assert_eq!(categorize_activity("unknown-category"), "Unknown");
    }
}
