//! Calendar data transformation utilities

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;

use crate::sources::google::types::{Event, EventTime};

/// Transform event data for different use cases
pub struct EventTransformer;

impl EventTransformer {
    /// Extract start datetime from event
    pub fn get_start_time(event: &Event) -> Option<DateTime<Utc>> {
        event.start.as_ref().and_then(|start| {
            Self::event_time_to_datetime(start)
        })
    }

    /// Extract end datetime from event
    pub fn get_end_time(event: &Event) -> Option<DateTime<Utc>> {
        event.end.as_ref().and_then(|end| {
            Self::event_time_to_datetime(end)
        })
    }

    /// Convert EventTime to DateTime
    fn event_time_to_datetime(event_time: &EventTime) -> Option<DateTime<Utc>> {
        if let Some(dt) = &event_time.date_time {
            return Some(*dt);
        }

        if let Some(date_str) = &event_time.date {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                return Some(date.and_hms_opt(0, 0, 0)?.and_utc());
            }
        }

        None
    }

    /// Calculate event duration in minutes
    pub fn get_duration_minutes(event: &Event) -> Option<i64> {
        let start = Self::get_start_time(event)?;
        let end = Self::get_end_time(event)?;
        Some(end.signed_duration_since(start).num_minutes())
    }

    /// Check if event is all-day
    pub fn is_all_day(event: &Event) -> bool {
        event.start.as_ref()
            .map(|s| s.date.is_some() && s.date_time.is_none())
            .unwrap_or(false)
    }

    /// Check if event is recurring
    pub fn is_recurring(event: &Event) -> bool {
        event.recurrence.is_some() || event.recurring_event_id.is_some()
    }

    /// Extract attendee emails
    pub fn get_attendee_emails(event: &Event) -> Vec<String> {
        event.attendees.as_ref()
            .map(|attendees| {
                attendees.iter()
                    .filter_map(|a| a.email.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Transform event for analytics
    pub fn to_analytics_format(event: &Event) -> Value {
        serde_json::json!({
            "event_id": event.id,
            "title": event.summary,
            "start_time": Self::get_start_time(event),
            "end_time": Self::get_end_time(event),
            "duration_minutes": Self::get_duration_minutes(event),
            "is_all_day": Self::is_all_day(event),
            "is_recurring": Self::is_recurring(event),
            "attendee_count": event.attendees.as_ref().map(|a| a.len()).unwrap_or(0),
            "has_location": event.location.is_some(),
            "status": event.status,
        })
    }
}