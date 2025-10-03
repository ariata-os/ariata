//! Google API type definitions

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Google Calendar List response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarListResponse {
    pub items: Vec<Calendar>,
    pub next_page_token: Option<String>,
}

/// Google Calendar
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: String,
    pub summary: String,
    pub description: Option<String>,
    pub time_zone: Option<String>,
    pub selected: Option<bool>,
    pub primary: Option<bool>,
}

/// Calendar Events response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsResponse {
    pub items: Vec<Event>,
    pub next_sync_token: Option<String>,
    pub next_page_token: Option<String>,
}

/// Calendar Event
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub etag: String,
    pub kind: String,
    pub status: Option<String>,
    pub html_link: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: Option<EventTime>,
    pub end: Option<EventTime>,
    pub recurrence: Option<Vec<String>>,
    pub recurring_event_id: Option<String>,
    pub organizer: Option<Person>,
    pub attendees: Option<Vec<Attendee>>,
    pub created: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
}

/// Event time (can be date or datetime)
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventTime {
    pub date: Option<String>,
    pub date_time: Option<DateTime<Utc>>,
    pub time_zone: Option<String>,
}

/// Person (organizer)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Person {
    pub email: Option<String>,
    pub display_name: Option<String>,
}

/// Attendee
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Attendee {
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub response_status: Option<String>,
    pub optional: Option<bool>,
}