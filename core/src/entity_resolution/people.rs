//! People Resolution via Calendar Attendees
//!
//! Resolves calendar attendees to canonical person entities.
//!
//! ## Process
//!
//! 1. Fetch calendar events in time window
//! 2. Extract attendee emails from TEXT[] column
//! 3. Match against entities_person by email
//! 4. Create new person entities for unknowns
//! 5. Update calendar metadata with resolved person IDs

use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use super::TimeWindow;

/// Resolve people from calendar attendees in time window
///
/// Returns the number of people resolved.
pub async fn resolve_people(db: &Database, window: TimeWindow) -> Result<usize> {
    tracing::info!(
        start = %window.start,
        end = %window.end,
        "Resolving people from calendar attendees"
    );

    // 1. Fetch calendar events in window
    let calendar_events = fetch_calendar_events(db, window).await?;

    if calendar_events.is_empty() {
        tracing::debug!("No calendar events to process");
        return Ok(0);
    }

    tracing::debug!(
        event_count = calendar_events.len(),
        "Fetched calendar events"
    );

    // 2. Process each event: resolve attendees and update calendar
    let mut total_people_resolved = 0;
    for event in calendar_events {
        match resolve_and_link_event_attendees(db, &event).await {
            Ok(count) => total_people_resolved += count,
            Err(e) => {
                tracing::warn!(
                    event_id = %event.id,
                    error = %e,
                    "Failed to resolve attendees for event"
                );
            }
        }
    }

    tracing::info!(
        people_resolved = total_people_resolved,
        "People resolution completed"
    );

    Ok(total_people_resolved)
}

/// Calendar event with attendees
#[derive(Debug)]
struct CalendarEvent {
    id: Uuid,
    attendee_identifiers: Vec<String>,
}

/// Fetch calendar events in time window
async fn fetch_calendar_events(db: &Database, window: TimeWindow) -> Result<Vec<CalendarEvent>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            attendee_identifiers
        FROM data.praxis_calendar
        WHERE start_time >= $1
          AND start_time < $2
          AND array_length(attendee_identifiers, 1) > 0
        "#,
        window.start,
        window.end
    )
    .fetch_all(db.pool())
    .await?;

    let events = rows
        .into_iter()
        .map(|row| CalendarEvent {
            id: row.id,
            attendee_identifiers: row.attendee_identifiers.unwrap_or_default(),
        })
        .collect();

    Ok(events)
}

/// Resolve all attendees for an event and update the calendar record
///
/// Returns the number of unique people resolved.
async fn resolve_and_link_event_attendees(db: &Database, event: &CalendarEvent) -> Result<usize> {
    if event.attendee_identifiers.is_empty() {
        return Ok(0);
    }

    // Resolve each attendee email to person entity
    let mut person_ids = Vec::new();
    let mut unique_people = std::collections::HashSet::new();

    for email in &event.attendee_identifiers {
        let email_lower = email.to_lowercase();

        match resolve_or_create_person(db, &email_lower).await {
            Ok(person_id) => {
                person_ids.push(person_id);
                unique_people.insert(person_id);
            }
            Err(e) => {
                tracing::warn!(
                    email = %email,
                    event_id = %event.id,
                    error = %e,
                    "Failed to resolve person for attendee"
                );
            }
        }
    }

    if person_ids.is_empty() {
        return Ok(0);
    }

    // Update calendar event with resolved person IDs
    sqlx::query!(
        r#"
        UPDATE data.praxis_calendar
        SET attendee_person_ids = $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
        &person_ids,
        event.id
    )
    .execute(db.pool())
    .await?;

    tracing::debug!(
        event_id = %event.id,
        people_count = unique_people.len(),
        "Linked attendees to calendar event"
    );

    Ok(unique_people.len())
}

/// Resolve email to person entity (or create if new)
///
/// Returns the person entity ID.
async fn resolve_or_create_person(db: &Database, email: &str) -> Result<Uuid> {
    // Check if person exists with this email
    let existing = sqlx::query!(
        r#"
        SELECT id
        FROM data.entities_person
        WHERE $1 = ANY(email_addresses)
        LIMIT 1
        "#,
        email
    )
    .fetch_optional(db.pool())
    .await?;

    if let Some(row) = existing {
        tracing::debug!(
            email = %email,
            person_id = %row.id,
            "Found existing person entity"
        );
        return Ok(row.id);
    }

    // Create new person entity
    let canonical_name = extract_name_from_email(email);

    let person_id = sqlx::query!(
        r#"
        INSERT INTO data.entities_person (
            canonical_name,
            email_addresses
        ) VALUES (
            $1, $2
        )
        RETURNING id
        "#,
        canonical_name,
        &vec![email.to_string()]
    )
    .fetch_one(db.pool())
    .await?
    .id;

    tracing::info!(
        email = %email,
        person_id = %person_id,
        canonical_name = %canonical_name,
        "Created new person entity"
    );

    Ok(person_id)
}

/// Extract name from email (simple heuristic)
///
/// Examples:
/// - adam.jace@example.com → "Adam Jace"
/// - john.doe@company.co → "John Doe"
/// - user123@domain.com → "user123"
fn extract_name_from_email(email: &str) -> String {
    let local_part = email.split('@').next().unwrap_or(email);

    // Split by dot or underscore
    let parts: Vec<&str> = local_part.split(&['.', '_'][..]).collect();

    if parts.len() > 1 {
        // Capitalize each part
        parts
            .iter()
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        // Just return the local part as-is
        local_part.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_name_from_email() {
        assert_eq!(extract_name_from_email("adam.jace@example.com"), "Adam Jace");
        assert_eq!(extract_name_from_email("john_doe@company.co"), "John Doe");
        assert_eq!(extract_name_from_email("user123@domain.com"), "user123");
        assert_eq!(extract_name_from_email("single@test.com"), "single");
    }
}
