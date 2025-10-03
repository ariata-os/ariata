//! Calendar event processing logic

use serde_json::json;
use chrono::Utc;

use crate::{
    error::Result,
    sources::SourceRecord,
};

use crate::sources::google::types::Event;

/// Calendar event processor
pub struct CalendarProcessor {
    source_name: String,
}

impl CalendarProcessor {
    /// Create a new calendar processor
    pub fn new(source_name: &str) -> Self {
        Self {
            source_name: source_name.to_string(),
        }
    }

    /// Process a list of events into source records
    pub fn process_events(&self, events: Vec<Event>, calendar_id: &str) -> Result<Vec<SourceRecord>> {
        let mut records = Vec::with_capacity(events.len());

        for event in events {
            records.push(self.event_to_record(event, calendar_id));
        }

        Ok(records)
    }

    /// Convert a single event to a source record
    fn event_to_record(&self, event: Event, calendar_id: &str) -> SourceRecord {
        SourceRecord {
            id: event.id.clone(),
            source: self.source_name.clone(),
            timestamp: event.updated.unwrap_or_else(Utc::now),
            data: json!({
                "id": event.id,
                "summary": event.summary,
                "description": event.description,
                "location": event.location,
                "start": event.start,
                "end": event.end,
                "status": event.status,
                "organizer": event.organizer,
                "attendees": event.attendees,
                "recurrence": event.recurrence,
                "recurring_event_id": event.recurring_event_id,
                "html_link": event.html_link,
            }),
            metadata: Some(json!({
                "calendar_id": calendar_id,
                "etag": event.etag,
                "kind": event.kind,
            })),
        }
    }

    /// Filter events based on criteria
    pub fn filter_events(&self, events: Vec<Event>, include_cancelled: bool) -> Vec<Event> {
        events.into_iter()
            .filter(|e| {
                if !include_cancelled && e.status.as_ref().map_or(false, |s| s == "cancelled") {
                    return false;
                }
                true
            })
            .collect()
    }
}