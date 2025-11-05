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

/// Block children response
#[derive(Debug, Deserialize)]
pub struct BlockChildrenResponse {
    pub results: Vec<Block>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// Block object
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Block {
    pub id: String,
    pub created_time: DateTime<Utc>,
    pub last_edited_time: DateTime<Utc>,
    pub has_children: bool,
    pub archived: bool,
    #[serde(rename = "type")]
    pub block_type: String,

    // Block type-specific fields
    pub paragraph: Option<BlockContent>,
    pub heading_1: Option<BlockContent>,
    pub heading_2: Option<BlockContent>,
    pub heading_3: Option<BlockContent>,
    pub bulleted_list_item: Option<BlockContent>,
    pub numbered_list_item: Option<BlockContent>,
    pub to_do: Option<ToDoBlock>,
    pub toggle: Option<BlockContent>,
    pub code: Option<CodeBlock>,
    pub quote: Option<BlockContent>,
    pub callout: Option<CalloutBlock>,
    pub child_page: Option<ChildPageBlock>,
}

/// Generic block content (paragraphs, headings, lists, etc.)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockContent {
    pub rich_text: Vec<RichText>,
    #[serde(default)]
    pub color: String,
}

/// To-do block
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToDoBlock {
    pub rich_text: Vec<RichText>,
    pub checked: Option<bool>,
    #[serde(default)]
    pub color: String,
}

/// Code block
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CodeBlock {
    pub rich_text: Vec<RichText>,
    pub language: String,
    #[serde(default)]
    pub caption: Vec<RichText>,
}

/// Callout block
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CalloutBlock {
    pub rich_text: Vec<RichText>,
    #[serde(default)]
    pub color: String,
}

/// Child page block
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChildPageBlock {
    pub title: String,
}

/// Rich text
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RichText {
    pub plain_text: String,
    pub href: Option<String>,
    #[serde(default)]
    pub annotations: Annotations,
}

/// Text annotations
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Annotations {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub strikethrough: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub code: bool,
}