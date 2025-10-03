//! Google API type definitions

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsResponse {
    pub items: Vec<Event>,
    pub next_sync_token: Option<String>,
    pub next_page_token: Option<String>,
}

/// Calendar Event
#[derive(Debug, Deserialize, Clone, Serialize)]
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
    pub created: Option<String>,
    pub updated: Option<String>,
    pub creator: Option<Person>,
    pub organizer: Option<Person>,
    pub attendees: Option<Vec<Attendee>>,
    pub conference_data: Option<ConferenceData>,
}

/// Event time (can be date or dateTime)
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTime {
    pub date: Option<String>,       // For all-day events
    pub date_time: Option<String>,  // For timed events
    pub time_zone: Option<String>,
}

/// Person (creator, organizer)
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub email: Option<String>,
    pub display_name: Option<String>,
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
}

/// Event attendee
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attendee {
    pub email: String,
    pub display_name: Option<String>,
    pub response_status: Option<String>,  // accepted, declined, tentative, needsAction
    pub optional: Option<bool>,
    pub organizer: Option<bool>,
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
}

/// Conference data for virtual meetings
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceData {
    pub conference_id: Option<String>,
    pub conference_solution: Option<ConferenceSolution>,
    pub entry_points: Option<Vec<EntryPoint>>,
}

/// Conference solution (Meet, Zoom, etc)
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceSolution {
    pub key: Option<ConferenceKey>,
    pub name: Option<String>,  // "Google Meet", "Zoom", etc
    pub icon_uri: Option<String>,
}

/// Conference key type
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceKey {
    #[serde(rename = "type")]
    pub key_type: String,  // "hangoutsMeet", etc
}

/// Conference entry point (URL, phone, etc)
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryPoint {
    pub entry_point_type: String,  // "video", "phone", etc
    pub uri: Option<String>,       // Meeting URL
    pub label: Option<String>,     // Display label
    pub pin: Option<String>,       // Meeting PIN if any
}