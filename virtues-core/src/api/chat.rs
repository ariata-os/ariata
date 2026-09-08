//! Chat API with SSE streaming
//!
//! Implements the AI SDK v6 UI Message Stream Protocol for chat completions.
//! Protocol uses JSON events with "type" field:
//!   - text-start: marks beginning of text block
//!   - text-delta: incremental text content
//!   - text-end: marks end of text block
//!   - reasoning-start/delta/end: for thinking tokens
//!   - error: error events
//!
//! Requires header: x-vercel-ai-ui-message-stream: v1
//!
//! Streams responses through virtues-api for budget enforcement and usage tracking.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    Json,
};
use chrono::Utc;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::StreamExt;

use crate::agent::{AgentConfig, AgentEvent, AgentLoop};
use crate::api::chat_usage::{record_chat_usage, UsageData};
use crate::api::chats::{append_message, ChatMessage, ToolCall};
use crate::api::compaction::{build_context_for_llm, compact_chat, CompactionOptions};
use crate::api::token_estimation::ContextStatus;
use crate::middleware::auth::AuthUser;
use crate::server::yjs::YjsState;
use crate::tools::ToolContext;
use crate::types::Timestamp;
use tokio_util::sync::CancellationToken;

// ============================================================================
// Cancellation State
// ============================================================================

/// Shared state for tracking active chat requests that can be cancelled
#[derive(Clone, Default)]
pub struct ChatCancellationState {
    /// Map of chat_id -> cancellation token for active requests
    tokens: Arc<std::sync::RwLock<std::collections::HashMap<String, CancellationToken>>>,
}

impl ChatCancellationState {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Register a new chat request and get its cancellation token
    pub fn register(&self, chat_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        // Recover from poisoned lock - the data is still valid
        let mut guard = self.tokens.write().unwrap_or_else(|e| e.into_inner());
        guard.insert(chat_id.to_string(), token.clone());
        token
    }

    /// Cancel an active chat request
    pub fn cancel(&self, chat_id: &str) -> bool {
        // Recover from poisoned lock - the data is still valid
        let guard = self.tokens.read().unwrap_or_else(|e| e.into_inner());
        if let Some(token) = guard.get(chat_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// Remove a chat request (called when stream completes)
    pub fn remove(&self, chat_id: &str) {
        // Recover from poisoned lock - the data is still valid
        let mut guard = self.tokens.write().unwrap_or_else(|e| e.into_inner());
        guard.remove(chat_id);
    }

    /// Check if a chat has an active request
    pub fn is_active(&self, chat_id: &str) -> bool {
        // Recover from poisoned lock - the data is still valid
        let guard = self.tokens.read().unwrap_or_else(|e| e.into_inner());
        guard.contains_key(chat_id)
    }
}

// ============================================================================
// Types
// ============================================================================

/// Active page context for AI page editing
#[derive(Debug, Deserialize)]
pub struct ActivePageContext {
    /// Bound page ID for editing
    pub page_id: Option<String>,
    /// Page title (for better LLM context)
    pub page_title: Option<String>,
    /// Current content from Yjs document (source of truth for edits)
    pub content: Option<String>,
}

/// Chat request from frontend
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<UIMessage>,
    #[serde(rename = "chatId")]
    pub chat_id: String,
    /// The person's per-turn pick from the picker, if they made one.
    ///
    /// Absent — the ordinary case — means "whatever this turn's slot resolves
    /// to", decided by the box in `model_choice::resolve_turn_model`. A client
    /// is never required to know a model id, and an unpinned chat is not
    /// frozen to the model it opened with. Empty string counts as absent; see
    /// that module for why shipped clients depend on it.
    #[serde(default)]
    pub model: Option<String>,
    #[serde(rename = "agentId", default = "default_agent")]
    pub agent_id: String,
    /// Optional client-generated message ID for idempotency
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    /// The Notebook (room) this chat lives in. Stored on the chat and inlined into
    /// the system prompt as a salience lens (name + memo + member URLs).
    #[serde(rename = "notebookId", default)]
    pub notebook_id: Option<String>,
    /// Optional active page context for AI page editing
    #[serde(rename = "activePage")]
    pub active_page: Option<ActivePageContext>,
    /// Optional Gemini thought signature for subsequent tool calls
    #[serde(rename = "thoughtSignature")]
    pub thought_signature: Option<String>,
    /// User's timezone (IANA format, e.g., "America/Los_Angeles")
    #[serde(default)]
    pub timezone: Option<String>,
    /// AI persona for system prompt customization (per-chat)
    #[serde(default = "default_persona")]
    pub persona: String,
    /// Agent mode controlling tool availability (agent, chat, research)
    #[serde(rename = "agentMode", default = "default_agent_mode")]
    pub agent_mode: String,
    /// Retrieval scope for notebook chats: "open" (default — whole graph,
    /// notebook items up-weighted) or "scoped" (grounded — items only).
    /// Ignored without a notebook_id.
    #[serde(rename = "chatMode", default = "default_chat_mode")]
    pub chat_mode: String,
    /// A temporary ("ghost") chat: nothing about it is written to the box.
    /// No chat row, no messages, no usage row. Its history arrives on the
    /// request every turn, because the box holds none.
    ///
    /// The client has sent this flag since ghost mode shipped and promised
    /// "never persisted" in its UI, while the box, which had no field to
    /// read, created the row and stored every message anyway. The only
    /// thing that still records a ghost turn is `app_ai_calls`: cost and
    /// token counts, no content, no chat id.
    #[serde(default)]
    pub temporary: bool,
}

/// A ghost chat's history, as the client sent it. The box stores nothing for
/// a temporary chat, so the wire is the only transcript. Text parts become
/// content; tool parts ride along in `parts` for the converter downstream.
fn ghost_history(messages: &[UIMessage]) -> Vec<ChatMessage> {
    messages
        .iter()
        .filter(|m| m.role == "user" || m.role == "assistant")
        .map(|m| {
            let content = m.content.clone().unwrap_or_else(|| {
                m.parts
                    .as_ref()
                    .map(|parts| {
                        parts
                            .iter()
                            .filter_map(|p| match p {
                                UIPart::Text { text } => Some(text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default()
            });
            ChatMessage {
                id: m.id.clone(),
                role: m.role.clone(),
                content,
                timestamp: Timestamp::now(),
                model: None,
                provider: None,
                agent_id: None,
                parts: m.parts.clone(),
                tool_calls: None,
                reasoning: None,
                intent: None,
                subject: None,
                thought_signature: None,
            }
        })
        .filter(|m| !m.content.is_empty() || m.parts.is_some())
        .collect()
}

fn default_chat_mode() -> String {
    "open".to_string()
}

fn default_agent() -> String {
    "auto".to_string()
}

fn default_persona() -> String {
    "default".to_string()
}

fn default_agent_mode() -> String {
    "chat".to_string()
}

/// UI Message format (AI SDK v6)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIMessage {
    pub id: Option<String>,
    pub role: String,
    #[serde(default)]
    pub parts: Option<Vec<UIPart>>,
    // Legacy format support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// UI Part types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum UIPart {
    Text {
        text: String,
    },
    Reasoning {
        text: String,
    },
    #[serde(rename = "tool-invocation")]
    ToolInvocation {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        /// Tool name - defaults to empty string if not provided (AI SDK may omit it)
        #[serde(rename = "toolName", default)]
        tool_name: String,
        #[serde(default)]
        input: serde_json::Value,
        #[serde(default)]
        state: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<serde_json::Value>,
    },
    #[serde(rename = "tool-web_search")]
    ToolWebSearch {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        /// Tool name - defaults to "web_search" since we know the type
        #[serde(rename = "toolName", default = "default_web_search_tool_name")]
        tool_name: String,
        #[serde(default)]
        input: serde_json::Value,
        #[serde(default)]
        state: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<serde_json::Value>,
    },
    /// Checkpoint from conversation compaction
    #[serde(rename = "checkpoint")]
    Checkpoint {
        /// Summary version number
        version: i32,
        /// Number of messages that were summarized
        messages_summarized: i32,
        /// The summary text (XML structured)
        summary: String,
        /// When the checkpoint was created
        timestamp: String,
    },
    /// File attachment (image / PDF / audio) — matches the AI SDK v6 file part.
    /// `url` is a data URL (base64) so it round-trips to the provider and renders
    /// on reload without a separate authenticated fetch.
    #[serde(rename = "file")]
    File {
        #[serde(rename = "mediaType", default)]
        media_type: String,
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
    #[serde(other)]
    Unknown,
}

/// Default tool name for web_search variant when toolName is missing from JSON
fn default_web_search_tool_name() -> String {
    "web_search".to_string()
}

/// Streaming event types (AI SDK v6 UI Message Stream Protocol)
///
/// These must exactly match the AI SDK's expected schema (strictObject validation).
/// See: https://sdk.vercel.ai/docs/ai-sdk-ui/stream-protocol
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StreamEvent {
    // Text streaming
    TextStart {
        id: String,
    },
    TextDelta {
        id: String,
        delta: String,
    },
    TextEnd {
        id: String,
    },

    // Reasoning/thinking tokens
    ReasoningStart {
        id: String,
    },
    ReasoningDelta {
        id: String,
        delta: String,
    },
    ReasoningEnd {
        id: String,
    },

    // Tool input streaming (AI SDK v6 format)
    #[serde(rename = "tool-input-start")]
    ToolInputStart {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
    },
    #[serde(rename = "tool-input-delta")]
    ToolInputDelta {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "inputTextDelta")]
        input_text_delta: String,
    },
    #[serde(rename = "tool-input-available")]
    ToolInputAvailable {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        input: serde_json::Value,
    },

    // Tool output (AI SDK v6: tool-output-available)
    #[serde(rename = "tool-output-available")]
    ToolOutputAvailable {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        output: serde_json::Value,
    },

    // Error handling
    Error {
        #[serde(rename = "errorText")]
        error_text: String,
    },

    // Custom event to sync thought signature to client
    #[serde(rename = "thought-signature")]
    ThoughtSignature {
        signature: String,
    },

    // The interview's write_it_up finished: tell the client which page to open
    // beside the chat. A data part rather than the tool part, because tool
    // parts are pushed into a message's parts array MUTABLY mid-turn and
    // effects watching the messages array provably never see them arrive;
    // onData fires deterministically.
    #[serde(rename = "narrative-document-ready")]
    NarrativeDocumentReady {
        #[serde(rename = "pageId")]
        page_id: String,
    },

    // Deep Research subagent status (data event for the live panel)
    #[serde(rename = "subagent-status")]
    SubagentStatus {
        #[serde(rename = "dispatchId")]
        dispatch_id: u64,
        #[serde(rename = "subagentId")]
        subagent_id: u32,
        title: String,
        model: String,
        status: String,
        tokens: u32,
    },

    // Checkpoint event emitted after auto-compaction
    #[serde(rename = "checkpoint")]
    Checkpoint {
        /// Message ID for the checkpoint
        id: String,
        /// Summary version number
        version: i32,
        /// Number of messages that were summarized
        #[serde(rename = "messagesSummarized")]
        messages_summarized: i32,
        /// The summary text (XML structured)
        summary: String,
        /// When the checkpoint was created
        timestamp: String,
    },
}

// ============================================================================
// AI SDK v6 Data Event Types
// ============================================================================

/// AI SDK v6 data event wrapper for custom events
/// Custom events must use "data-*" prefix to be properly handled by DefaultChatTransport
#[derive(Debug, Serialize)]
struct DataEvent<T: Serialize> {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    data: T,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    transient: bool,
}

/// Checkpoint data payload for AI SDK v6 data event
#[derive(Debug, Serialize)]
struct CheckpointData {
    version: i32,
    #[serde(rename = "messagesSummarized")]
    messages_summarized: i32,
    summary: String,
    timestamp: String,
}

/// Thought signature data payload for AI SDK v6 data event
#[derive(Debug, Serialize)]
struct ThoughtSignatureData {
    signature: String,
}

/// Narrative-document-ready payload for AI SDK v6 data event
#[derive(Debug, Serialize)]
struct NarrativeDocumentData {
    #[serde(rename = "pageId")]
    page_id: String,
}

/// Subagent status payload for AI SDK v6 data event (live Deep Research panel)
#[derive(Debug, Serialize)]
struct SubagentStatusData {
    #[serde(rename = "dispatchId")]
    dispatch_id: u64,
    #[serde(rename = "subagentId")]
    subagent_id: u32,
    title: String,
    model: String,
    /// "thinking" | "done" | "failed"
    status: String,
    tokens: u32,
}

/// Chat error response
#[derive(Debug, Serialize)]
pub struct ChatError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

// ============================================================================
// SSE Types
// ============================================================================

type SseEvent = axum::response::sse::Event;

/// Fetch the latest checkpoint message from a chat and convert to StreamEvent
async fn get_latest_checkpoint(pool: &PgPool, chat_id: &str) -> Option<StreamEvent> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"
        SELECT id, parts, created_at
        FROM app_chat_messages
        WHERE chat_id = $1 AND role = 'checkpoint'
        ORDER BY sequence_num DESC
        LIMIT 1
        "#,
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await
    .ok()??;

    let id: String = row.get("id");
    let parts_json: Option<String> = row.get("parts");
    let created_at: String = row.get("created_at");

    // Parse parts JSON to extract checkpoint data
    let parts: Vec<UIPart> = parts_json
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();

    // Find checkpoint part
    for part in parts {
        if let UIPart::Checkpoint {
            version,
            messages_summarized,
            summary,
            timestamp,
        } = part
        {
            return Some(StreamEvent::Checkpoint {
                id,
                version,
                messages_summarized,
                summary,
                // Use checkpoint timestamp, falling back to created_at if empty
                timestamp: if timestamp.is_empty() { created_at } else { timestamp },
            });
        }
    }

    None
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Safely serialize a stream event to JSON
/// Custom events (checkpoint, thought-signature) are wrapped in AI SDK v6 data-* format
fn serialize_event(event: &StreamEvent) -> String {
    match event {
        // Wrap checkpoint events in AI SDK v6 data event format
        StreamEvent::Checkpoint { id, version, messages_summarized, summary, timestamp } => {
            let wrapper = DataEvent {
                event_type: "data-checkpoint".to_string(),
                id: Some(id.clone()),
                data: CheckpointData {
                    version: *version,
                    messages_summarized: *messages_summarized,
                    summary: summary.clone(),
                    timestamp: timestamp.clone(),
                },
                transient: false, // Checkpoint should persist in message parts
            };
            serde_json::to_string(&wrapper).unwrap_or_else(|e| {
                tracing::error!("Failed to serialize checkpoint event: {}", e);
                r#"{"type":"error","errorText":"Serialization error"}"#.to_string()
            })
        }
        // Wrap thought signature events in AI SDK v6 data event format
        StreamEvent::ThoughtSignature { signature } => {
            let wrapper = DataEvent {
                event_type: "data-thought-signature".to_string(),
                id: None,
                data: ThoughtSignatureData { signature: signature.clone() },
                transient: true, // Ephemeral - only needed during streaming session
            };
            serde_json::to_string(&wrapper).unwrap_or_else(|e| {
                tracing::error!("Failed to serialize thought-signature event: {}", e);
                r#"{"type":"error","errorText":"Serialization error"}"#.to_string()
            })
        }
        // Wrap the narrative-document signal in AI SDK v6 data event format
        // (transient — a reload reconstructs the document from its page, and
        // replaying the auto-open on old chats would be wrong).
        StreamEvent::NarrativeDocumentReady { page_id } => {
            let wrapper = DataEvent {
                event_type: "data-narrative-document".to_string(),
                id: None,
                data: NarrativeDocumentData { page_id: page_id.clone() },
                transient: true,
            };
            serde_json::to_string(&wrapper).unwrap_or_else(|e| {
                tracing::error!("Failed to serialize narrative-document event: {}", e);
                r#"{"type":"error","errorText":"Serialization error"}"#.to_string()
            })
        }
        // Wrap subagent status in AI SDK v6 data event format (transient — live panel only)
        StreamEvent::SubagentStatus { dispatch_id, subagent_id, title, model, status, tokens } => {
            let wrapper = DataEvent {
                event_type: "data-subagent".to_string(),
                id: None,
                data: SubagentStatusData {
                    dispatch_id: *dispatch_id,
                    subagent_id: *subagent_id,
                    title: title.clone(),
                    model: model.clone(),
                    status: status.clone(),
                    tokens: *tokens,
                },
                transient: true,
            };
            serde_json::to_string(&wrapper).unwrap_or_else(|e| {
                tracing::error!("Failed to serialize subagent event: {}", e);
                r#"{"type":"error","errorText":"Serialization error"}"#.to_string()
            })
        }
        // All other events use standard serde serialization
        _ => serde_json::to_string(event).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize stream event: {}", e);
            r#"{"type":"error","errorText":"Internal serialization error"}"#.to_string()
        }),
    }
}

/// Maximum characters for page content in system prompt
/// ~10K chars ≈ 2.5K tokens, leaving room for rest of context
const MAX_PAGE_CONTENT_CHARS: usize = 10_000;

/// Build narrative identity content for the system prompt.
///
/// The user's telos document — values, character, aspirations — for the system
/// prompt. Empty string when unset, which is the ordinary case.
///
/// **Budget: ~2k tokens, and the reason is behavioural, not economic.** NI sits
/// in the SYSTEM prompt, so it is prompt-cached and the marginal cost per turn
/// is a cache read; 5k would be affordable. What a longer document costs is
/// precision. The prompt around this already spends four paragraphs telling the
/// model to hold it lightly — "the fastest way to lose trust is to psychoanalyse
/// a shopping list" — and every extra paragraph is more surface for a spurious
/// connection to a routine question. A longer NI does not make the assistant
/// understand you better; it makes it perform understanding more often.
///
/// Truncation happens at a PARAGRAPH boundary. The previous version cut at 800
/// characters mid-word, which would have fed the model a sentence that stops
/// in the middle and invited it to complete the thought itself. (It never fired
/// in practice: the table has no rows on a real box, so NI has been the empty
/// string in every prompt since it shipped. Fixing it is a build, not a repair.)
async fn build_narrative_identity(pool: &PgPool) -> String {
    // The DOCUMENT itself — "In your own words", the page the person edits.
    // There is deliberately no abridged copy between them and the assistant:
    // the old capsule (wiki_narrative_identity) drifted from the document and
    // was caught inventing standing directives. One artifact, read whole,
    // truncated at a paragraph boundary if the person writes past the budget.
    match crate::api::wiki_articles::get_article_prose(
        pool,
        "narrative_identity",
        crate::api::narrative_draft::NAR_IDENTITY_ID,
    )
    .await
    {
        Ok(Some(prose)) if !prose.content.trim().is_empty() => {
            truncate_to_budget(&prose.content, NI_BUDGET_CHARS)
        }
        _ => String::new(),
    }
}

/// ~2k tokens. English averages near four characters per token, and this is a
/// ceiling rather than a target.
const NI_BUDGET_CHARS: usize = 8_000;

/// Cut at the last paragraph break inside the budget; if there is no break to
/// find, cut at the last sentence end; only then fall back to a hard cut.
///
/// Never mid-word: a document that ends mid-sentence reads to the model as a
/// thought it should finish, and this one is about who a person is.
fn truncate_to_budget(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let cut: String = text.chars().take(budget).collect();
    if let Some(i) = cut.rfind("\n\n") {
        return cut[..i].trim_end().to_string();
    }
    if let Some(i) = cut.rfind(['.', '!', '?']) {
        return cut[..=i].to_string();
    }
    match cut.rfind(char::is_whitespace) {
        Some(i) => cut[..i].to_string(),
        None => cut,
    }
}

/// The rules block — `wiki_rules`, grouped by kind.
///
/// THIS TABLE WAS WRITTEN AND NEVER READ. From 0101 until now, `wiki_rules` (and
/// `wiki_standing_order` before it) was populated by the interview's last
/// question and consumed by nothing: the box obeyed no rule anyone had written,
/// while the interview told them in as many words that "what you write here
/// stops being context and becomes a rule." This function is what makes that
/// sentence true.
///
/// Grouped rather than listed flat because `avoid` and `defend` need opposite
/// handling, and the prompt can only give them opposite handling if it can tell
/// them apart.
///
/// Empty string when there are no rules, so the caller can omit the section.
/// Errors are swallowed to an empty string on purpose — a database blip must
/// degrade to "no rules injected" rather than taking down chat. That is the
/// safe direction only because the alternative is worse; it does mean a failed
/// read silently un-enforces, which is why it is logged.
async fn build_rules(pool: &PgPool) -> String {
    let rows = match sqlx::query_as::<_, (String, String)>(
        "SELECT kind, rule FROM wiki_rules WHERE active ORDER BY created_at",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "rules: read failed — none will be enforced this turn");
            return String::new();
        }
    };

    let mut avoid: Vec<&str> = Vec::new();
    let mut defend: Vec<&str> = Vec::new();
    for (kind, rule) in &rows {
        let rule = rule.trim();
        if rule.is_empty() {
            continue;
        }
        match kind.as_str() {
            "defend" => defend.push(rule),
            // Anything unrecognised is treated as `avoid`. The CHECK constraint
            // makes that unreachable today, and if a third kind is ever added,
            // the conservative reading is the one that cannot cause harm.
            _ => avoid.push(rule),
        }
    }

    let mut out = String::new();
    if !avoid.is_empty() {
        out.push_str("Never raise these unless they do:\n");
        for r in avoid {
            out.push_str(&format!("- {r}\n"));
        }
    }
    if !defend.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("Help them hold to these:\n");
        for r in defend {
            out.push_str(&format!("- {r}\n"));
        }
    }
    out
}

/// Build system prompt with dynamic context and personalization.
///
/// Assembles: identity → persona → narrative_identity → tools → datetime → user_context → active_page.
/// Loads user name, assistant name, persona, and narrative identity from profiles.
async fn build_system_prompt(
    pool: &PgPool,
    active_page: Option<&ActivePageContext>,
    timezone: Option<&str>,
    agent_mode: &str,
    persona_id: &str,
    notebook_id: Option<&str>,
) -> String {
    use crate::api::assistant_profile::get_assistant_name;
    use crate::api::profile::get_display_name;

    // Load personalization from profiles (with fallbacks)
    let assistant_name = get_assistant_name(pool).await.unwrap_or_else(|_| "Ari".to_string());
    let user_name = get_display_name(pool).await.unwrap_or_else(|_| "there".to_string());

    // The narrative interview is a different room entirely: no tools, no
    // persona, no data context, no narrative-identity injection (the document
    // this conversation exists to create). Its prompt stands alone.
    if agent_mode == "interview" {
        // The person's reply count is the one fact about progress the box can
        // vouch for; the prompt reads it as a floor on what can be covered.
        let their_replies = crate::api::narrative_draft::their_reply_count(pool)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "interview reply count unavailable; prompt says 0");
                0
            });
        return crate::agent::prompt::build_interview_prompt(&assistant_name, &user_name, their_replies);
    }

    build_system_prompt_blocks(
        pool,
        active_page,
        timezone,
        agent_mode,
        persona_id,
        notebook_id,
        &assistant_name,
        &user_name,
    )
    .await
    .0
}

/// The registry: every prompt section as a named block, rendered in list
/// order by `prompt_blocks::assemble`. This is the current order verbatim —
/// the formula's reorder (rules last, quantized clock, cache breakpoint) is
/// a deliberate later slice, and it happens by editing THIS list.
#[allow(clippy::too_many_arguments)]
async fn build_system_prompt_blocks(
    pool: &PgPool,
    active_page: Option<&ActivePageContext>,
    timezone: Option<&str>,
    agent_mode: &str,
    persona_id: &str,
    notebook_id: Option<&str>,
    assistant_name: &str,
    user_name: &str,
) -> (String, Vec<crate::agent::prompt_blocks::RenderedBlock>) {
    use crate::agent::prompt::build_personalized_prompt;
    use crate::agent::prompt_blocks::{assemble, Author, Block, BlockMeta, Cadence, Mood};
    use crate::api::personas::get_persona_content;

    let blocks: Vec<Block<'_>> = vec![
        // The fused head: base identity + persona + narrative identity + tool
        // guidance + mode, still one string from build_personalized_prompt.
        // Splits into <character>/<narrative_identity>/<tools> in the reorder
        // slice.
        Block {
            meta: BlockMeta { tag: "base", author: Author::System, mood: Mood::Declarative, rung: 40, cadence: Cadence::Slow },
            body: Box::pin(async move {
                let persona_content = get_persona_content(pool, persona_id).await.ok().flatten();
                let narrative_identity = build_narrative_identity(pool).await;
                Some(build_personalized_prompt(
                    assistant_name,
                    user_name,
                    persona_id,
                    persona_content.as_deref(),
                    agent_mode,
                    &narrative_identity,
                ))
            }),
        },
        // The precedence ladder, stated once. GPT-5-era guidance and our own
        // formula doc agree: unresolved hierarchy ambiguity measurably
        // degrades compliance, so the ranking is text the model can cite,
        // not an emergent property of ordering.
        Block {
            meta: BlockMeta { tag: "precedence", author: Author::System, mood: Mood::Declarative, rung: 40, cadence: Cadence::Static },
            body: Box::pin(async move {
                Some(crate::agent::prompt_blocks::precedence_line().to_string())
            }),
        },
        // The machine's memory: per-note rows the person can read and edit
        // (Settings), rendered with ids so the update_memory ops can name
        // them. Three lanes per docs/narrative-identity.md: facts of their
        // world, their preferred manner, their practices.
        Block {
            meta: BlockMeta { tag: "memory", author: Author::Machine, mood: Mood::Declarative, rung: 50, cadence: Cadence::Session },
            body: Box::pin(async move {
                let memories = match crate::api::assistant_memories::list_memories(pool).await {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("[chat] memory omitted from the prompt: {e}");
                        return None;
                    }
                };
                if memories.is_empty() {
                    return None;
                }
                let mut out = String::from(
                    "\n\n<memory>\nWhat you have learned alongside {user}, in notes you keep (they can read and edit these in Settings; a note marked [theirs] is in their words — never revise or retire it). Use silently: reference when relevant, never recite unprompted.\n",
                );
                out = out.replace("{user}", user_name);
                for lane in ["facts", "manner", "practices"] {
                    let in_lane: Vec<_> = memories.iter().filter(|m| m.lane == lane).collect();
                    if in_lane.is_empty() {
                        continue;
                    }
                    out.push_str(&format!("<{lane}>\n"));
                    for m in in_lane {
                        let theirs = if m.author == "human" { " [theirs]" } else { "" };
                        out.push_str(&format!("- (#{}){} {}\n", m.id, theirs, m.body));
                    }
                    out.push_str(&format!("</{lane}>\n"));
                }
                out.push_str("</memory>");
                Some(out)
            }),
        },
        // The computed present — clock, place, today's spine, calendar,
        // recent people (with entity ids), live threads, last night's sleep,
        // narrated recent days, connected sources. Deterministic, SQL-only,
        // budgeted by fixed caps; the quarter-hour clock is computed ONCE and
        // every line derives from the same instant. Replaces the old
        // <datetime> + <user_context> pair (formula slice 4).
        Block {
            meta: BlockMeta { tag: "circumstances", author: Author::Computed, mood: Mood::Declarative, rung: 30, cadence: Cadence::Quantized },
            body: Box::pin(async move {
                let now = Utc::now();
                let floored = now
                    - chrono::Duration::minutes(i64::from(
                        now.format("%M").to_string().parse::<u32>().unwrap_or(0) % 15,
                    ));
                crate::api::circumstances::build_circumstances(pool, timezone, floored).await
            }),
        },
        // The active Notebook (room) as a salience lens: its name, catch-up
        // memo, and member URLs. This is the room the chat lives in.
        Block {
            meta: BlockMeta { tag: "active_notebook", author: Author::Ui, mood: Mood::Declarative, rung: 45, cadence: Cadence::Session },
            body: Box::pin(async move {
                match notebook_id {
                    Some(id) => build_notebook_context(pool, id).await,
                    None => None,
                }
            }),
        },
        // The open page's live content (Yjs is the source of truth).
        Block {
            meta: BlockMeta { tag: "active_context", author: Author::Ui, mood: Mood::Declarative, rung: 45, cadence: Cadence::PerTurn },
            body: Box::pin(async move { build_active_page_block(active_page) }),
        },
        // The enforceable half — LAST, nearest the conversation (moved
        // 2026-08-28; RULES_PROMPT's own placement note predicted exactly
        // this lever). Constraint adherence tracks recency, and rules are
        // the one block that must hold at 1-in-1000. Sitting after the
        // volatile tail also means they are never cached and never bust
        // anything — a few hundred deliberately re-sent tokens. Absent
        // entirely when there are no rules: an empty <rules> block would
        // teach the model that the section is usually noise.
        Block {
            meta: BlockMeta { tag: "rules", author: Author::User, mood: Mood::Imperative, rung: 100, cadence: Cadence::Slow },
            body: Box::pin(async move {
                let rules = build_rules(pool).await;
                (!rules.is_empty()).then(|| {
                    crate::agent::prompt::RULES_PROMPT
                        .replace("{user_name}", user_name)
                        .replace("{rules}", &rules)
                })
            }),
        },
    ];

    assemble(blocks).await
}

/// Render the open-page section, if a page is open.
fn build_active_page_block(active_page: Option<&ActivePageContext>) -> Option<String> {
    let ctx = active_page?;
    let page_id = ctx.page_id.as_ref()?;
    let title = ctx.page_title.as_deref().unwrap_or("Untitled");

    Some(match &ctx.content {
        Some(content) => {
            // Truncate large content to avoid consuming too much context
            let (content_display, truncation_note) = if content.chars().count() > MAX_PAGE_CONTENT_CHARS {
                let truncated_content: String = content.chars().take(MAX_PAGE_CONTENT_CHARS).collect();
                let remaining = content.chars().count() - MAX_PAGE_CONTENT_CHARS;
                let truncated = format!(
                    "{}...\n\n[Content truncated - {} more characters]",
                    truncated_content,
                    remaining
                );
                (truncated, " The content shown is truncated. Call get_page_content for the complete document before making edits.")
            } else {
                (content.clone(), "")
            };

            format!(
                "\n\n<active_context>\nThe user has \"{}\" (id: {}) open for editing.\n\n<current_content>\n{}\n</current_content>\n\nUse the edit_page tool to make changes. The 'find' parameter locates text, 'replace' provides the new text. For a full rewrite, set find to empty string. Edits are applied immediately via real-time sync.{}\n</active_context>",
                title, page_id, content_display, truncation_note
            )
        }
        None => format!(
            "\n\n<active_context>\nThe user has \"{}\" (id: {}) open for editing. Use get_page_content to read it first, then edit_page to make changes.\n</active_context>",
            title, page_id
        ),
    })
}

/// Test-only view of the assembled prompt, so audits in other modules can
/// assert on what the model is actually sent rather than re-deriving it.
#[cfg(test)]
pub(crate) async fn build_system_prompt_for_audit(pool: &PgPool) -> String {
    build_system_prompt(pool, None, Some("America/Chicago"), "default", "default", None).await
}

/// Maximum member URLs to inline for a Notebook before truncating.
const MAX_NOTEBOOK_ITEMS_INLINED: usize = 100;

/// Build a context block for the active Notebook (room) the chat lives in.
/// Returns None if the Notebook can't be loaded.
async fn build_notebook_context(pool: &PgPool, notebook_id: &str) -> Option<String> {
    let detail = match crate::api::notebooks::get_notebook(pool, notebook_id).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("[chat] failed to load active notebook {}: {}", notebook_id, e);
            return None;
        }
    };

    let mut out = String::new();
    out.push_str(&format!(
        "\n\n<active_notebook name=\"{}\">",
        escape_attr(&detail.notebook.name),
    ));

    if let Some(instr) = detail.notebook.instructions.as_deref() {
        if !instr.is_empty() {
            out.push_str(&format!(
                "\n  <instructions>{}</instructions>",
                escape_attr(instr)
            ));
        }
    }

    if let Some(memo) = detail.notebook.current_status.as_deref() {
        if !memo.is_empty() {
            out.push_str(&format!("\n  <memo>{}</memo>", escape_attr(memo)));
        }
    }

    let total = detail.items.len();
    let shown: Vec<_> = detail
        .items
        .iter()
        .take(MAX_NOTEBOOK_ITEMS_INLINED)
        .collect();

    // Resolve every member in one batch before writing the block. A bare URL
    // is unreadable: the model cannot tell a screenshot from a spreadsheet
    // without spending tool calls to find out, and the room it is standing in
    // should not require an investigation.
    let urls: Vec<String> = shown.iter().map(|i| i.url.clone()).collect();
    let resolved = crate::api::refs::resolve_refs(pool, &urls).await;

    for item in shown {
        out.push_str(&format!("\n  <member url=\"{}\"", escape_attr(&item.url)));
        if let Some(r) = resolved.get(&item.url) {
            out.push_str(&format!(" title=\"{}\"", escape_attr(&r.title)));
            out.push_str(&format!(" kind=\"{}\"", escape_attr(&r.kind)));
            if let Some(mime) = r.mime.as_deref() {
                out.push_str(&format!(" mime=\"{}\"", escape_attr(mime)));
            }
            // Only meaningful for files: says whether semantic_search can see
            // this member's contents, so a dead end is known up front rather
            // than discovered three tool calls in.
            if let Some(text) = r.text.as_deref() {
                out.push_str(&format!(" text=\"{}\"", escape_attr(text)));
            }
        }
        // `library` grounds chat; `manuscript` is the user's own draft, kept
        // out of retrieval so it is never cited back at them; `pin` is
        // navigation. Without this the three were indistinguishable here, and
        // a draft read exactly like a source.
        out.push_str(&format!(" role=\"{}\"/>", escape_attr(&item.role)));
    }
    if total > MAX_NOTEBOOK_ITEMS_INLINED {
        out.push_str(&format!(
            "\n  <!-- {} more members not shown -->",
            total - MAX_NOTEBOOK_ITEMS_INLINED
        ));
    }

    out.push_str("\n</active_notebook>");

    let preamble = "\n\n<active_notebook_preamble>\nThis chat lives in the Notebook (room) below — a collection the user returns to (a project, pet, hobby, goal, or topic). Treat its members as high-salience: they are the user's actively curated focus for this room. <instructions>, if present, are standing directions for how you should behave in this notebook — follow them. <memo>, if present, is a catch-up note about the notebook's current state. Members are also boosted in semantic search while this notebook is active.\n\nThe member list below IS the notebook's contents — it is already complete (up to the cap noted at its end). When the user refers to something \"in this notebook,\" match it here first; do not go looking for the notebook's contents with other tools.\n\nEach member carries what it is: `title`, `kind`, and `role`. `role=\"library\"` grounds this chat; `role=\"manuscript\"` is the user's own draft, deliberately excluded from retrieval — never cite it back at them as a source; `role=\"pin\"` is navigation only. Files also carry `text`: `indexed` means its contents are searchable, `pending` means extraction has not finished yet, and `none` means no text was extracted — searching for its contents will find nothing, so say so plainly rather than reporting an empty search as an absence of the thing.\n</active_notebook_preamble>";

    Some(format!("{}{}", preamble, out))
}

/// Minimal XML attribute escaping for the inlined context block.
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ============================================================================
// Handler
// ============================================================================

/// POST /api/chat - Stream chat completion
///
/// Requires authentication. Routes through virtues-api for budget enforcement.
pub async fn chat_handler(
    State(pool): State<PgPool>,
    State(yjs_state): State<YjsState>,
    State(cancel_state): State<ChatCancellationState>,
    _user: AuthUser,
    Json(mut request): Json<ChatRequest>,
) -> Response {
    // The narrative interview is a MODE OF THE CHAT, decided by the chat id —
    // never by what the client sent. Any surface that opens this conversation
    // gets the interviewer (its standalone prompt, zero tools); no client can
    // opt the interview into tools by sending a different agentMode.
    if request.chat_id == crate::api::narrative_draft::INTERVIEW_CHAT_ID {
        request.agent_mode = "interview".to_string();
    }

    // Which model answers. One door — the box decides, from the mode and the
    // owner's pin; the request's `model` is a per-turn override and nothing
    // else. See `model_choice` for why the interview refuses that override,
    // and why an empty string means "I did not choose".
    let model = match crate::api::model_choice::resolve_turn_model(
        &pool,
        request.model.as_deref(),
        &request.agent_mode,
    )
    .await
    {
        Ok(m) => m,
        // Name the id we rejected. The version of this that listed 244
        // allowed ids and never the offending one cost a day of debugging.
        Err(crate::error::Error::InvalidInput(detail)) => {
            tracing::warn!(
                requested = ?request.model,
                agent_mode = %request.agent_mode,
                "rejected per-turn model"
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(ChatError {
                    error: "Invalid model".to_string(),
                    details: Some(detail),
                }),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to resolve the model for this turn");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChatError {
                    error: "Failed to resolve model".to_string(),
                    details: Some(e.to_string()),
                }),
            )
                .into_response();
        }
    };
    // Deliberately NOT written back onto `request`: the resolved address has
    // one home from here down, threaded as an argument. `request.model` stays
    // what the client sent, which is the only thing it ever meant.


    // Use client-provided message ID for idempotency, or generate one
    let msg_id = request
        .message_id
        .clone()
        .unwrap_or_else(|| format!("msg_{}", generate_id()));

    // The June-era chat onboarding is DELETED (2026-09-01) — onboarding is
    // the founder's letter + Home's getting-started page, and the assistant's
    // first conversation is an ordinary one. onboarding_status still gates
    // those surfaces; the chat no longer reads it.

    // Ensure chat exists - use ON CONFLICT DO NOTHING to handle race conditions
    let chat_id_str = request.chat_id.clone();
    let title = {
        let raw_title = request
            .messages
            .iter()
            .find(|m| m.role == "user")
            .and_then(|m| {
                m.content.clone().or_else(|| {
                    m.parts.as_ref().and_then(|p| {
                        p.iter().find_map(|p| match p {
                            UIPart::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                    })
                })
            })
            .unwrap_or_else(|| "New conversation".to_string());

        if raw_title.chars().count() > 50 {
            let t: String = raw_title.chars().take(47).collect();
            format!("{}...", t)
        } else {
            raw_title
        }
    };

    // A ghost chat has no row. Everything below that reads or writes
    // app_chats / app_chat_messages / app_chat_usage branches on this.
    let temporary = request.temporary;

    // Use ON CONFLICT DO NOTHING to handle concurrent requests for same chat
    // Returns rows_affected = 1 if inserted, 0 if already exists
    let chat_was_created = if temporary {
        false
    } else {
        match sqlx::query("INSERT INTO app_chats (id, title, message_count) VALUES ($1, $2, 0) ON CONFLICT (id) DO NOTHING")
            .bind(&chat_id_str)
            .bind(&title)
            .execute(&pool)
            .await
        {
            Ok(result) => result.rows_affected() > 0,
            Err(e) => {
                tracing::error!("Failed to create chat: {}", e);
                false
            }
        }
    };

    if chat_was_created {
        if let Err(e) =
            crate::api::notebooks::set_chat_notebook(&pool, &chat_id_str, request.notebook_id.as_deref()).await
        {
            tracing::warn!("Failed to set chat notebook: {}", e);
        }
    }

    // Save the last user message to the chat
    if let Some(last_user_msg) = request.messages.iter().rev().find(|m| m.role == "user") {
        // Normal flow: save the last user message from the request
        let user_content = last_user_msg.content.clone().unwrap_or_else(|| {
            last_user_msg
                .parts
                .as_ref()
                .map(|p| {
                    p.iter()
                        .filter_map(|p| match p {
                            UIPart::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default()
        });

        let user_message = ChatMessage {
            id: None,
            role: "user".to_string(),
            content: user_content,
            timestamp: Timestamp::now(),
            model: None,
            provider: None,
            agent_id: None,
            parts: last_user_msg.parts.clone(),
            tool_calls: None,
            reasoning: None,
            intent: None,
            subject: None,
            thought_signature: None,
        };

        if temporary {
            // Ghost: the message lives in the client's tab and nowhere else.
        } else if let Err(e) = append_message(&pool, request.chat_id.clone(), user_message).await {
            tracing::error!("Failed to save user message: {}", e);
        }
    }

    // Check if compaction is needed before sending to LLM. A ghost chat has
    // no usage row to read and no summary to write, so it never compacts.
    let compaction_needed = if temporary {
        false
    } else {
        let compaction_status =
            crate::api::chat_usage::check_compaction_needed(&pool, request.chat_id.clone(), &model)
                .await;
        // Pass compaction_needed flag to stream - compaction will run inside stream
        // and emit a checkpoint event for real-time UI updates
        matches!(compaction_status, Ok(ContextStatus::Critical))
    };

    use sqlx::Row;

    // Load chat from DB and build context with compaction summary
    let (conversation_summary, summary_up_to_index): (Option<String>, i64) = if temporary {
        (None, 0)
    } else {
        match sqlx::query(
            r#"SELECT conversation_summary, summary_up_to_index
               FROM app_chats WHERE id = $1"#,
        )
        .bind(&chat_id_str)
        .fetch_one(&pool)
        .await
        {
            Ok(row) => (row.get("conversation_summary"), row.get("summary_up_to_index")),
            Err(e) => {
                tracing::error!("Failed to load chat: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ChatError {
                        error: "Failed to load chat".to_string(),
                        details: Some(e.to_string()),
                    }),
                )
                    .into_response();
            }
        }
    };

    // The transcript: the box's rows, or for a ghost chat the client's copy.
    let messages: Vec<ChatMessage> = if temporary {
        ghost_history(&request.messages)
    } else {
        // Load messages from normalized table
        let message_rows = match sqlx::query(
            r#"
            SELECT
                id, role, content, created_at, model, provider, agent_id,
                reasoning, tool_calls, intent, subject, thought_signature, parts
            FROM app_chat_messages
            WHERE chat_id = $1
            ORDER BY sequence_num ASC
            "#,
        )
        .bind(&chat_id_str)
        .fetch_all(&pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("Failed to load messages for chat {}: {}", chat_id_str, e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ChatError {
                        error: "Failed to load messages".to_string(),
                        details: Some(e.to_string()),
                    }),
                )
                    .into_response();
            }
        };

        // Convert rows to ChatMessage
        message_rows
            .into_iter()
            .map(|msg| {
                let id: String = msg.get("id");
                let role: String = msg.get("role");
                let content: String = msg.get("content");
                let created_at: Timestamp = msg.get("created_at");
                let model: Option<String> = msg.get("model");
                let provider: Option<String> = msg.get("provider");
                let agent_id: Option<String> = msg.get("agent_id");
                let reasoning: Option<String> = msg.get("reasoning");
                // Columns are jsonb; read as serde_json::Value, not String
                let tool_calls_raw: Option<serde_json::Value> = msg.get("tool_calls");
                let intent_raw: Option<serde_json::Value> = msg.get("intent");
                let subject: Option<String> = msg.get("subject");
                let thought_signature: Option<String> = msg.get("thought_signature");
                let parts_raw: Option<serde_json::Value> = msg.get("parts");

                // Parse JSON fields
                let tool_calls = tool_calls_raw.and_then(|t| serde_json::from_value(t).ok());
                let intent = intent_raw.and_then(|i| serde_json::from_value(i).ok());
                let parts = parts_raw.and_then(|p| serde_json::from_value(p).ok());

                ChatMessage {
                    id: Some(id),
                    role,
                    content,
                    timestamp: created_at,
                    model,
                    provider,
                    agent_id,
                    parts,
                    reasoning,
                    tool_calls,
                    intent,
                    subject,
                    thought_signature,
                }
            })
            .collect()
    };

    // Resolve the chat's room from the persisted row (single source of truth) so
    // the active-notebook context always matches the binding, even if a stale client
    // sends a different per-message notebookId. The create path above already bound a
    // new chat from request.notebook_id, so the row is current by now.
    // Decoded as `Option<String>` on purpose: `app_chats.notebook_id` is nullable
    // (and the FK is ON DELETE SET NULL), so an unbound chat legitimately reads
    // NULL. Scalar-typing it as `String` would make that NULL a decode *error* —
    // which is what the old `.ok()` was really swallowing, alongside every real
    // query failure. A swallow here is not cosmetic: None reads as "not in a
    // notebook", so a broken query silently unscopes a scoped chat — retrieval
    // stops being hard-filtered and the answer contract below is dropped.
    // A ghost chat has no row to read it from; the request is the binding.
    let effective_notebook_id: Option<String> = if temporary {
        request.notebook_id.clone()
    } else {
        match sqlx::query_scalar::<_, Option<String>>(
        r#"SELECT notebook_id FROM app_chats WHERE id = $1"#,
    )
    .bind(&chat_id_str)
    .fetch_optional(&pool)
    .await
    {
        // Outer None = no such row, inner None = bound to no notebook.
        Ok(notebook_id) => notebook_id.flatten(),
        Err(e) => {
            tracing::error!("Failed to resolve notebook for chat {}: {}", chat_id_str, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChatError {
                    error: "Failed to resolve chat notebook".to_string(),
                    details: Some(e.to_string()),
                }),
            )
                .into_response();
        }
        }
    };

    // Build system prompt with active page context, timezone, personalization, and agent mode
    let mut system_prompt = build_system_prompt(&pool, request.active_page.as_ref(), request.timezone.as_deref(), &request.agent_mode, &request.persona, effective_notebook_id.as_deref()).await;
    // Scoped (grounded) chat: retrieval is hard-filtered to the notebook's
    // items (ScopeMode::Exclusive in ToolContext); this line sets the matching
    // answer contract. Only meaningful inside a notebook.
    if request.chat_mode == "scoped" && effective_notebook_id.is_some() {
        system_prompt.push_str(
            "\n\nSCOPED CHAT: this conversation is grounded in the current notebook's \
             materials only. Retrieval is restricted to them. Answer ONLY from what \
             retrieval returns, citing each load-bearing claim with its ref link. If \
             the materials don't cover the question, say so plainly — do not answer \
             from general knowledge.",
        );
    }

    // Build context using compaction summary if available
    let api_messages = build_context_for_llm(
        &messages,
        conversation_summary.as_deref(),
        summary_up_to_index as usize,
        Some(&system_prompt),
    );

    let stream = {
        create_agent_stream(
            pool,
            yjs_state,
            cancel_state,
            request,
            model,
            api_messages,
            msg_id,
            compaction_needed,
        )
    };

    // AI SDK v6 requires this header for UI Message Stream Protocol
    let mut response = Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new())
        .into_response();

    response.headers_mut().insert(
        axum::http::header::HeaderName::from_static("x-vercel-ai-ui-message-stream"),
        axum::http::HeaderValue::from_static("v1"),
    );

    // We can't easily send the signature in headers for a streaming response
    // because it's discovered DURING the stream.
    // However, the frontend can extract it from the stream itself if we emit a special event.

    response.into_response()
}

/// Create the SSE stream using the AgentLoop for tool execution
fn create_agent_stream(
    pool: PgPool,
    yjs_state: YjsState,
    cancel_state: ChatCancellationState,
    request: ChatRequest,
    // Already resolved by `model_choice::resolve_turn_model` — passed in
    // rather than re-read off the request so the stream cannot disagree with
    // what the handler decided, or fall back to a default of its own.
    model: String,
    api_messages: Vec<serde_json::Value>,
    msg_id: String,
    compaction_needed: bool,
) -> Pin<Box<dyn Stream<Item = Result<SseEvent, Infallible>> + Send>> {
    let chat_id = request.chat_id.clone();
    // Copied out for the stream block below, which reads `request` for a
    // few fields and must not persist a ghost turn either.
    let temporary = request.temporary;
    let agent_id = request.agent_id.clone();

    Box::pin(async_stream::stream! {
        // Register cancellation token for this chat
        let cancel_token = cancel_state.register(&chat_id);

        // Run compaction BEFORE the agent loop if needed, and emit checkpoint event
        if compaction_needed {
            tracing::info!(
                chat_id = %chat_id,
                "Context critical, auto-compacting chat"
            );
            let compaction_options = CompactionOptions {
                model_id: Some(model.clone()),
                ..Default::default()
            };
            match compact_chat(&pool, chat_id.clone(), compaction_options).await {
                Ok(_) => {
                    // Fetch the checkpoint message that was just created
                    if let Some(checkpoint_event) = get_latest_checkpoint(&pool, &chat_id).await {
                        yield Ok(SseEvent::default().data(serialize_event(&checkpoint_event)));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        chat_id = %chat_id,
                        error = %e,
                        "Auto-compaction failed, continuing with full context"
                    );
                }
            }
        }

        // Determine max_steps based on agent mode
        // - deep_research: 50 (read-only, needs more exploration)
        // - council: 40 (gate + 1-2 dispatch rounds + synthesis; bounded)
        // - chat / default: 20 (conversational, full tool access, multi-turn)
        let max_steps = match request.agent_mode.as_str() {
            "deep_research" => 50,
            "council" => 40, // gate + 1-2 dispatch rounds + synthesis; bounded
            _ => 20,         // "chat" or default
        };

        // Create AgentLoop with YjsState for real-time page editing
        let agent = AgentLoop::new_with_yjs(pool.clone(), yjs_state)
        .with_config(AgentConfig {
            max_steps,
            tool_timeout: std::time::Duration::from_secs(30),
            parallel_tools: true,
        });

        // Side-channel for live Deep Research subagent status. The dispatch_subagents tool sends
        // worker updates on `subagent_tx`; the select! loop below drains `subagent_rx` and streams
        // them to the panel while the tool is still executing.
        let (subagent_tx, mut subagent_rx) =
            tokio::sync::mpsc::channel::<crate::tools::SubagentUpdate>(64);

        // Per-turn budget of Deep Research workers, shared across repeated dispatches so the
        // orchestrator can't fan out without bound over its 50-step loop.
        let worker_budget = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(12));

        // Build tool context from request
        let context = ToolContext {
            page_id: request.active_page.as_ref().and_then(|p| p.page_id.clone()),
            user_id: None,
            notebook_id: request.notebook_id.clone(),
            scope_mode: if request.chat_mode == "scoped" {
                crate::search::ScopeMode::Exclusive
            } else {
                crate::search::ScopeMode::Weighted
            },
            chat_id: Some(request.chat_id.clone()),
            applet_id: None,
            subagent_tx: Some(subagent_tx),
            cancel_token: Some(cancel_token.clone()),
            worker_budget: Some(worker_budget),
        };

        let tools = crate::tools::get_tools_for_agent_mode(&request.agent_mode);

        // Send text-start event
        let start_event = StreamEvent::TextStart { id: msg_id.clone() };
        yield Ok(SseEvent::default().data(serialize_event(&start_event)));

        // Track accumulated content
        let mut full_content = String::new();
        let mut reasoning_content = String::new();
        let mut in_reasoning = false;
        // The whole turn streams as ONE text part (single TextStart/TextEnd), so
        // text emitted across agent steps would otherwise concatenate with no
        // separator ("…exact text.The earlier edit…"). When text resumes after a
        // tool call, insert a paragraph break so each narration reads on its own.
        let mut needs_text_break = false;

        // Token usage tracking
        let mut total_input_tokens: u32 = 0;
        let mut total_output_tokens: u32 = 0;
        let mut total_reasoning_tokens: u32 = 0;
        // Authoritative spend for this turn (sum of gateway-reported usage.cost
        // across every step), captured into app_ai_calls for the Usage tab.
        let mut total_cost_micros: i64 = 0;

        // Tool call tracking for persistence
        let mut all_tool_calls: Vec<ToolCall> = Vec::new();

        // Run the agent loop with cancellation support
        let mut agent_stream = agent.run(
            model.clone(),
            api_messages.clone(),
            tools,
            context,
            request.thought_signature.clone().or_else(|| {
                // Fallback: look for signature in the last assistant message of the history
                api_messages.iter().rev()
                    .filter_map(|m| m.get("thought_signature").and_then(|s| s.as_str()))
                    .next()
                    .map(|s| s.to_string())
            }),
            Some(cancel_token.clone()),
        );

        loop {
          tokio::select! {
            biased;
            // Live subagent status — drained even while dispatch_subagents is still executing.
            Some(update) = subagent_rx.recv() => {
                let ev = StreamEvent::SubagentStatus {
                    dispatch_id: update.dispatch_id,
                    subagent_id: update.id as u32,
                    title: update.title,
                    model: update.model,
                    status: update.status.as_str().to_string(),
                    tokens: update.tokens,
                };
                yield Ok(SseEvent::default().data(serialize_event(&ev)));
            }
            maybe_event = agent_stream.next() => {
              let event = match maybe_event {
                  Some(e) => e,
                  None => break,
              };
              match event {
                AgentEvent::TextDelta { content } => {
                    // End reasoning if we were in it
                    if in_reasoning {
                        in_reasoning = false;
                        let event = StreamEvent::ReasoningEnd { id: msg_id.clone() };
                        yield Ok(SseEvent::default().data(serialize_event(&event)));
                    }
                    // Text resuming after a tool call: break the paragraph so it
                    // doesn't butt against the previous segment's final sentence.
                    let delta = if needs_text_break
                        && !full_content.is_empty()
                        && !full_content.ends_with('\n')
                    {
                        format!("\n\n{}", content)
                    } else {
                        content
                    };
                    needs_text_break = false;
                    full_content.push_str(&delta);
                    let event = StreamEvent::TextDelta {
                        id: msg_id.clone(),
                        delta,
                    };
                    yield Ok(SseEvent::default().data(serialize_event(&event)));
                }

                AgentEvent::ReasoningDelta { content } => {
                    if !in_reasoning {
                        in_reasoning = true;
                        let event = StreamEvent::ReasoningStart { id: msg_id.clone() };
                        yield Ok(SseEvent::default().data(serialize_event(&event)));
                    }
                    reasoning_content.push_str(&content);
                    let event = StreamEvent::ReasoningDelta {
                        id: msg_id.clone(),
                        delta: content,
                    };
                    yield Ok(SseEvent::default().data(serialize_event(&event)));
                }

                AgentEvent::ToolCallStart { id, name, args } => {
                    // Any text that resumes after this tool call starts a new
                    // paragraph (see needs_text_break).
                    needs_text_break = true;
                    // Track tool call for persistence
                    all_tool_calls.push(ToolCall {
                        tool_name: name.clone(),
                        tool_call_id: Some(id.clone()),
                        arguments: args.clone().unwrap_or(serde_json::Value::Null),
                        result: None, // Will be populated by ToolCallResult
                        timestamp: Utc::now().to_rfc3339(),
                    });
                    // AI SDK v6: tool-input-start event
                    let event = StreamEvent::ToolInputStart {
                        tool_call_id: id,
                        tool_name: name,
                    };
                    yield Ok(SseEvent::default().data(serialize_event(&event)));
                }

                AgentEvent::ToolCallArgsPartial { id, args_delta } => {
                    // AI SDK v6: tool-input-delta event
                    let event = StreamEvent::ToolInputDelta {
                        tool_call_id: id,
                        input_text_delta: args_delta,
                    };
                    yield Ok(SseEvent::default().data(serialize_event(&event)));
                }

                AgentEvent::ToolCallArgsComplete { id, args } => {
                    // AI SDK v6: tool-input-available event (args parsing complete)
                    // Find the tool name from tracked tool calls
                    let tool_name = all_tool_calls.iter()
                        .find(|tc| tc.tool_call_id.as_deref() == Some(&id))
                        .map(|tc| tc.tool_name.clone())
                        .unwrap_or_default();
                    let event = StreamEvent::ToolInputAvailable {
                        tool_call_id: id,
                        tool_name,
                        input: args,
                    };
                    yield Ok(SseEvent::default().data(serialize_event(&event)));
                }

                AgentEvent::ToolCallResult { id, result, success: _, error: _ } => {
                    // Update the tracked tool call with the result
                    if let Some(tc) = all_tool_calls.iter_mut().find(|tc| tc.tool_call_id.as_deref() == Some(&id)) {
                        tc.result = Some(result.clone());
                    }
                    // The interview's finisher names the page the client
                    // should open beside the chat (see NarrativeDocumentReady).
                    let narrative_page = all_tool_calls
                        .iter()
                        .find(|tc| tc.tool_call_id.as_deref() == Some(&id))
                        .filter(|tc| tc.tool_name == "write_it_up")
                        .and_then(|_| result.get("document_page_id"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    // Bill nested Deep Research worker tokens to this chat's usage. The dispatch
                    // result carries aggregate worker token counts that the orchestrator's own
                    // Usage events don't include.
                    if let Some(usage) = result.get("usage") {
                        total_input_tokens += usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        total_output_tokens += usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    }
                    // AI SDK v6: tool-output-available event
                    let event = StreamEvent::ToolOutputAvailable {
                        tool_call_id: id,
                        output: result,
                    };
                    yield Ok(SseEvent::default().data(serialize_event(&event)));
                    if let Some(page_id) = narrative_page {
                        let event = StreamEvent::NarrativeDocumentReady { page_id };
                        yield Ok(SseEvent::default().data(serialize_event(&event)));
                    }
                }

                AgentEvent::Usage { prompt_tokens, completion_tokens, total_tokens: _, reasoning_tokens, cost_micros } => {
                    total_input_tokens += prompt_tokens;
                    total_output_tokens += completion_tokens;
                    if let Some(r) = reasoning_tokens {
                        total_reasoning_tokens += r;
                    }
                    if let Some(c) = cost_micros {
                        total_cost_micros += c;
                    }
                }

                AgentEvent::ThoughtSignature { signature } => {
                    let event = StreamEvent::ThoughtSignature { signature };
                    yield Ok(SseEvent::default().data(serialize_event(&event)));
                }

                AgentEvent::Error { message, code: _, recoverable: _ } => {
                    let event = StreamEvent::Error { error_text: message };
                    yield Ok(SseEvent::default().data(serialize_event(&event)));
                }

                // Events we don't need to forward to client
                AgentEvent::LoopStarted { .. } |
                AgentEvent::StepComplete { .. } |
                AgentEvent::MessageId { .. } |
                AgentEvent::Done { .. } => {}
              }
            }
          }
        }

        // Drain any subagent updates buffered after the agent loop ended.
        while let Ok(update) = subagent_rx.try_recv() {
            let ev = StreamEvent::SubagentStatus {
                dispatch_id: update.dispatch_id,
                subagent_id: update.id as u32,
                title: update.title,
                model: update.model,
                status: update.status.as_str().to_string(),
                tokens: update.tokens,
            };
            yield Ok(SseEvent::default().data(serialize_event(&ev)));
        }

        // End reasoning if we were in it
        if in_reasoning {
            let event = StreamEvent::ReasoningEnd { id: msg_id.clone() };
            yield Ok(SseEvent::default().data(serialize_event(&event)));
        }

        // Send text-end event
        let end_event = StreamEvent::TextEnd { id: msg_id.clone() };
        yield Ok(SseEvent::default().data(serialize_event(&end_event)));

        // Send [DONE] marker
        yield Ok(SseEvent::default().data("[DONE]"));

        // Save assistant message to chat
        if !full_content.is_empty() {
            let provider = model.split('/').next().unwrap_or("unknown").to_string();
            // Mark the message as user-stopped so the UI can show a "Stopped"
            // notice on reload (the partial content is kept either way).
            let was_cancelled = cancel_token.is_cancelled();
            let assistant_message = ChatMessage {
                id: None,
                role: "assistant".to_string(),
                content: full_content.clone(),
                timestamp: Timestamp::now(),
                model: Some(model.clone()),
                provider: Some(provider),
                agent_id: Some(agent_id),
                tool_calls: if all_tool_calls.is_empty() { None } else { Some(all_tool_calls.clone()) },
                reasoning: if reasoning_content.is_empty() { None } else { Some(reasoning_content.clone()) },
                intent: None,
                subject: if was_cancelled { Some("cancelled".to_string()) } else { None },
                thought_signature: None,
                parts: None,
            };

            if temporary {
                // Ghost: nothing written. The client keeps the turn in its tab.
            } else if let Err(e) = append_message(&pool, chat_id.clone(), assistant_message).await {
                tracing::error!("Failed to save assistant message: {}", e);
            }

            // Record token usage. `cost_micros` is the gateway's authoritative
            // figure — the same one recorded in app_ai_calls below, and the one
            // the wallet was actually debited for. No estimating.
            let usage_data = UsageData {
                input_tokens: total_input_tokens as i64,
                output_tokens: total_output_tokens as i64,
                reasoning_tokens: 0,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                cost_micros: Some(total_cost_micros),
            };

            if temporary {
                // Ghost: app_chat_usage keys on a chat row that does not exist.
                // app_ai_calls below still records cost and counts, no content.
            } else if let Err(e) = record_chat_usage(&pool, chat_id.clone(), &model, usage_data).await {
                tracing::warn!(
                    chat_id = %chat_id,
                    error = %e,
                    "Failed to record chat usage"
                );
            }

            // Box-local per-call cost log (authoritative gateway cost) for the
            // Usage/Telemetry tabs. Best-effort — never break the response.
            if let Err(e) = crate::api::ai_calls::record_ai_call(
                &pool,
                &crate::api::ai_calls::AiCall {
                    // Real feature bucket: chat | council | deep_research (these
                    // modes share this handler), so spend attributes correctly.
                    feature: request.agent_mode.clone(),
                    model: model.clone(),
                    prompt_tokens: total_input_tokens as i64,
                    completion_tokens: total_output_tokens as i64,
                    reasoning_tokens: total_reasoning_tokens as i64,
                    cost_micros: total_cost_micros,
                    // `stream()` diverts to the user's endpoint when BYO is
                    // set, and no upstream but our own gateway sends a cost
                    // trailer — so `total_cost_micros` is 0-as-unknown there,
                    // and the Usage tab must show tokens instead of "$0.00".
                    route: if crate::api::settings_byo::byo_is_active(&pool).await {
                        crate::api::ai_calls::Route::Byo
                    } else {
                        crate::api::ai_calls::Route::Wallet
                    },
                    applet_run_id: None,
                },
            )
            .await
            {
                tracing::warn!(chat_id = %chat_id, error = %e, "Failed to record ai_call");
            }
        }

        // Clean up cancellation token when stream ends
        cancel_state.remove(&chat_id);
    })
}

/// Generate a random ID for messages
fn generate_id() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 8] = rng.random();
    hex::encode(bytes)
}

// ============================================================================
// Cancel Handler
// ============================================================================

/// Request body for cancelling a chat
#[derive(Debug, Deserialize)]
pub struct CancelChatRequest {
    #[serde(rename = "chatId")]
    pub chat_id: String,
}

/// Response for cancel request
#[derive(Debug, Serialize)]
pub struct CancelChatResponse {
    pub cancelled: bool,
    pub message: String,
}

/// POST /api/chat/cancel - Cancel an in-progress chat request
///
/// Stops the agent loop for the specified chat, preserving any partial results.
pub async fn cancel_chat_handler(
    State(cancel_state): State<ChatCancellationState>,
    _user: AuthUser,
    Json(request): Json<CancelChatRequest>,
) -> impl IntoResponse {
    let cancelled = cancel_state.cancel(&request.chat_id);

    let response = CancelChatResponse {
        cancelled,
        message: if cancelled {
            "Chat request cancelled".to_string()
        } else {
            "No active request found for this chat".to_string()
        },
    };

    (StatusCode::OK, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rules block is the one place where a silent failure means the box
    /// raises a subject someone asked it never to raise. These tests exist
    /// because that failure is invisible from the outside: the prompt still
    /// builds, chat still answers, and nothing looks wrong.
    #[sqlx::test]
    async fn rules_are_grouped_by_kind(pool: PgPool) {
        sqlx::query(
            "INSERT INTO wiki_rules (id, rule, kind) VALUES
                ('r1', 'my father', 'avoid'),
                ('r2', 'the morning pages', 'defend'),
                ('r3', 'drinking', 'avoid')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let out = build_rules(&pool).await;

        // Each kind under its own heading, or the prompt cannot give them the
        // opposite handling they need.
        let avoid_at = out.find("Never raise").expect("avoid heading");
        let defend_at = out.find("Help them hold").expect("defend heading");
        assert!(avoid_at < defend_at, "avoid group comes first:\n{out}");
        assert!(out.contains("- my father"));
        assert!(out.contains("- drinking"));
        assert!(out.contains("- the morning pages"));

        // The defend rule must not be filed under avoid.
        let defend_block = &out[defend_at..];
        assert!(!defend_block.contains("my father"), "kinds bled:\n{out}");
    }

    /// No rules must render NOTHING, not an empty heading — a `<rules>` block
    /// that is usually empty teaches a model to skim past it.
    #[sqlx::test]
    async fn no_rules_renders_nothing(pool: PgPool) {
        assert_eq!(build_rules(&pool).await, "");
    }

    /// Inactive rules are withdrawn, not merely hidden from the review screen.
    #[sqlx::test]
    async fn inactive_rules_are_not_enforced(pool: PgPool) {
        sqlx::query("INSERT INTO wiki_rules (id, rule, kind, active) VALUES ('r1', 'my father', 'avoid', false)")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(build_rules(&pool).await, "");
    }

    /// THE TEST THAT WOULD HAVE CAUGHT THIS.
    ///
    /// `build_rules` passing proves the grouping; only assembling the whole
    /// prompt proves the block is actually wired in. From 0101 until 2026-08-17
    /// every unit around this was fine and the rule still never reached a model,
    /// because nothing asserted on the finished prompt.
    #[sqlx::test]
    async fn rules_reach_the_assembled_prompt(pool: PgPool) {
        sqlx::query("INSERT INTO wiki_rules (id, rule, kind) VALUES ('r1', 'my brother Tom', 'avoid')")
            .execute(&pool)
            .await
            .unwrap();

        let prompt = build_system_prompt_for_audit(&pool).await;
        assert!(prompt.contains("<rules>"), "rules block never reached the prompt");
        assert!(prompt.contains("my brother Tom"), "the rule text is missing:\n{prompt}");
    }

    /// And the block is absent when there is nothing to say.
    #[sqlx::test]
    async fn empty_rules_leave_no_block(pool: PgPool) {
        let prompt = build_system_prompt_for_audit(&pool).await;
        // The <precedence> block legitimately NAMES <rules>; what must be
        // absent is the rules block's own body.
        assert!(
            !prompt.contains("has marked some things as rules"),
            "empty rules block was rendered"
        );
    }

    /// The registry IS the render order. This pins the current order so a
    /// future edit to the block list is a deliberate, test-visible act — the
    /// reorder slice (rules last, per the formula doc) flips this assertion
    /// on purpose, and nothing reorders by accident.
    #[sqlx::test]
    async fn prompt_blocks_render_in_registry_order(pool: PgPool) {
        sqlx::query("INSERT INTO wiki_rules (id, kind, rule) VALUES ('rule_t1', 'avoid', 'x')")
            .execute(&pool)
            .await
            .unwrap();
        let (prompt, rendered) = build_system_prompt_blocks(
            &pool, None, Some("America/Chicago"), "default", "default", None, "Ari",
            "Adam",
        )
        .await;

        let tags: Vec<&str> = rendered.iter().map(|r| r.tag).collect();
        // Blocks with no data render nothing; the ones that DO render must
        // appear in registry order, with the fused head first.
        assert_eq!(tags.first(), Some(&"base"), "the fused head must render first");
        assert_eq!(tags.last(), Some(&"rules"), "rules must render last — nearest the conversation");
        assert!(tags.contains(&"rules"), "seeded rule missing from the assembly");
        assert!(tags.contains(&"circumstances"));
        let expected = ["base", "precedence", "new_user", "memory", "circumstances", "active_notebook", "active_context", "rules"];
        let mut last = 0usize;
        for t in &tags {
            let pos = expected.iter().position(|e| e == t).expect("unknown block tag");
            assert!(pos >= last, "block {t} rendered out of registry order: {tags:?}");
            last = pos;
        }
        // And the assembly is still one contiguous prompt, not fragments.
        assert!(prompt.contains("<rules>") && prompt.contains("<circumstances>"));
    }

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 16); // 8 bytes = 16 hex chars
    }
}


#[cfg(test)]
mod ni_budget_tests {
    use super::{truncate_to_budget, NI_BUDGET_CHARS};

    #[test]
    fn short_documents_are_untouched() {
        let t = "I value patience.\n\nI want to finish the boat.";
        assert_eq!(truncate_to_budget(t, NI_BUDGET_CHARS), t);
    }

    /// The point of the budget is to stop somewhere a reader would stop.
    #[test]
    fn cuts_at_a_paragraph_break_when_there_is_one() {
        let t = "First para.\n\nSecond para is long and would be cut.";
        assert_eq!(truncate_to_budget(t, 25), "First para.");
    }

    #[test]
    fn falls_back_to_a_sentence_end() {
        let t = "One sentence here. Then a much longer second sentence follows.";
        assert_eq!(truncate_to_budget(t, 30), "One sentence here.");
    }

    /// Never mid-word: a document that stops mid-sentence reads as a thought
    /// the model should finish, and this one is about who a person is.
    #[test]
    fn never_splits_a_word() {
        let t = "aaaa bbbb cccc dddddddddddddddddddd";
        let out = truncate_to_budget(t, 14);
        assert!(!out.ends_with("cc"), "cut inside a word: {out:?}");
        assert_eq!(out, "aaaa bbbb");
    }
}

/// Renders the real active-notebook block against a dev database, so the text
/// the model actually receives is inspected rather than assumed. Ignored by
/// default — CI has no box database.
///   cargo test -p virtues --lib api::chat::live_notebook -- --ignored --nocapture
#[cfg(test)]
mod live_notebook {
    use sqlx::PgPool;

    #[tokio::test]
    #[ignore]
    async fn the_block_names_every_member() {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://virtues:virtues@localhost:5432/virtues".to_string());
        let pool = PgPool::connect(&url).await.expect("dev database");

        let id: Option<String> = sqlx::query_scalar(
            "SELECT notebook_id FROM app_notebook_items
             GROUP BY notebook_id ORDER BY count(*) DESC LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("query");
        let Some(id) = id else {
            println!("no notebooks with members; nothing to render");
            return;
        };

        let block = super::build_notebook_context(&pool, &id)
            .await
            .expect("a context block");
        println!("\n{block}\n");

        // A member line carrying only a url is the bug this replaced.
        for line in block.lines().filter(|l| l.trim_start().starts_with("<member")) {
            assert!(line.contains("role="), "member without role: {line}");
            assert!(
                line.contains("title=") || line.contains("/home"),
                "member reached the prompt as a bare url: {line}"
            );
        }
    }
}

/// Measures the real system prompt against a dev database. Not an assertion of
/// correctness — an audit instrument, so the prompt's size and composition are
/// observed rather than estimated.
///   cargo test -p virtues --lib api::live_prompt_audit -- --ignored --nocapture
#[cfg(test)]
mod live_prompt_audit {
    use sqlx::PgPool;

    #[tokio::test]
    #[ignore]
    async fn measure_the_assembled_system_prompt() {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://virtues:virtues@localhost:5432/virtues".to_string());
        let pool = PgPool::connect(&url).await.expect("dev database");

        let notebook: Option<String> = sqlx::query_scalar(
            "SELECT notebook_id FROM app_notebook_items
             GROUP BY notebook_id ORDER BY count(*) DESC LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("query");

        for (label, nb) in [("no notebook", None), ("in a notebook", notebook.as_deref())] {
            let p = super::build_system_prompt(
                &pool,
                None,
                Some("America/Chicago"),
                "default",
                "default",
                nb,
            )
            .await;

            // ~4 chars/token is the usual English approximation; this is an
            // order-of-magnitude reading, not a billing figure.
            println!(
                "\n=== {label} ===\n{} chars  (~{} tokens)",
                p.len(),
                p.len() / 4
            );
            for tag in [
                "<persona",
                "<narrative_identity",
                "<memory>",
                "<datetime>",
                "<user_context>",
                "<identity>",
                "<recent_days>",
                "<connected_sources>",
                "<active_notebook",
                "<active_context>",
            ] {
                if let Some(i) = p.find(tag) {
                    // Crude section sizing: distance to the next top-level tag.
                    let rest = &p[i + tag.len()..];
                    let end = rest.find("\n\n<").map(|e| e + tag.len()).unwrap_or(p.len() - i);
                    println!("  {:<22} {:>6} chars", tag, end);
                }
            }
        }
    }
}


#[cfg(test)]
mod ghost_tests {
    use super::*;

    fn ui(role: &str, text: Option<&str>, parts: Option<Vec<UIPart>>) -> UIMessage {
        UIMessage { id: None, role: role.into(), parts, content: text.map(str::to_string) }
    }

    /// A ghost chat's transcript is exactly what the client sent: user and
    /// assistant turns, text drawn from parts when there is no content, and
    /// nothing else (no system rows, no empty rows).
    #[test]
    fn ghost_history_is_the_wire_and_only_the_wire() {
        let history = ghost_history(&[
            ui("system", Some("ignored"), None),
            ui("user", None, Some(vec![UIPart::Text { text: "hello".into() }, UIPart::Text { text: "there".into() }])),
            ui("assistant", Some("hi"), None),
            ui("user", None, None),
        ]);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[0].content, "hello\nthere");
        assert_eq!(history[1].content, "hi");
    }
}
