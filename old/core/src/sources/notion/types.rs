//! Notion API type definitions

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use serde_json::Value;

/// Search response
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<Page>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// Notion Page
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Page {
    pub id: String,
    pub created_time: DateTime<Utc>,
    pub last_edited_time: DateTime<Utc>,
    pub created_by: User,
    pub last_edited_by: User,
    pub parent: Parent,
    pub archived: bool,
    pub properties: Value,
    pub url: String,
}

/// Notion User
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub id: String,
    pub name: Option<String>,
}

/// Page parent
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum Parent {
    #[serde(rename = "database_id")]
    Database { database_id: String },
    #[serde(rename = "page_id")]
    Page { page_id: String },
    #[serde(rename = "workspace")]
    Workspace { workspace: bool },
}

/// Block object
#[derive(Debug, Deserialize, Serialize)]
pub struct Block {
    pub id: String,
    pub created_time: DateTime<Utc>,
    pub last_edited_time: DateTime<Utc>,
    pub has_children: bool,
    #[serde(rename = "type")]
    pub block_type: String,
    pub paragraph: Option<ParagraphBlock>,
}

/// Paragraph block
#[derive(Debug, Deserialize, Serialize)]
pub struct ParagraphBlock {
    pub rich_text: Vec<RichText>,
}

/// Rich text
#[derive(Debug, Deserialize, Serialize)]
pub struct RichText {
    pub plain_text: String,
    pub href: Option<String>,
}