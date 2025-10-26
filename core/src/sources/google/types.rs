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

// ============ Gmail Types ============

/// Gmail messages list response
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesListResponse {
    pub messages: Option<Vec<MessageRef>>,
    pub next_page_token: Option<String>,
    pub result_size_estimate: Option<i32>,
}

/// Message reference in list response
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRef {
    pub id: String,
    pub thread_id: String,
}

/// Gmail threads list response
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadsListResponse {
    pub threads: Option<Vec<ThreadRef>>,
    pub next_page_token: Option<String>,
    pub result_size_estimate: Option<i32>,
}

/// Thread reference in list response
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRef {
    pub id: String,
    pub snippet: Option<String>,
    pub history_id: Option<String>,
}

/// Full Gmail message
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub thread_id: String,
    pub label_ids: Option<Vec<String>>,
    pub snippet: Option<String>,
    pub history_id: Option<String>,
    pub internal_date: Option<String>,  // Milliseconds since epoch as string
    pub payload: Option<MessagePart>,
    pub size_estimate: Option<i32>,
    pub raw: Option<String>,  // Base64 encoded raw message
}

/// Message part (MIME structure)
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePart {
    pub part_id: Option<String>,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub headers: Option<Vec<MessageHeader>>,
    pub body: Option<MessageBody>,
    pub parts: Option<Vec<MessagePart>>,  // Recursive for multipart
}

/// Message header
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageHeader {
    pub name: String,
    pub value: String,
}

/// Message body
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageBody {
    pub attachment_id: Option<String>,
    pub size: i32,
    pub data: Option<String>,  // Base64url encoded
}

/// Gmail thread with messages
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: String,
    pub snippet: Option<String>,
    pub history_id: Option<String>,
    pub messages: Option<Vec<Message>>,
}

/// Gmail label
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub id: String,
    pub name: String,
    pub message_list_visibility: Option<String>,
    pub label_list_visibility: Option<String>,
    pub r#type: Option<String>,  // "system" or "user"
}

/// Gmail history record for incremental sync
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: String,
    pub messages: Option<Vec<Message>>,
    pub messages_added: Option<Vec<HistoryMessageAdded>>,
    pub messages_deleted: Option<Vec<HistoryMessageDeleted>>,
    pub labels_added: Option<Vec<HistoryLabelAdded>>,
    pub labels_removed: Option<Vec<HistoryLabelRemoved>>,
}

/// History message added
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessageAdded {
    pub message: Message,
}

/// History message deleted
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessageDeleted {
    pub message: MessageRef,
}

/// History label added
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryLabelAdded {
    pub message: MessageRef,
    pub label_ids: Vec<String>,
}

/// History label removed
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryLabelRemoved {
    pub message: MessageRef,
    pub label_ids: Vec<String>,
}

/// Gmail history response
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResponse {
    pub history: Option<Vec<HistoryRecord>>,
    pub next_page_token: Option<String>,
    pub history_id: Option<String>,
}