//! Conversation Compaction Module
//!
//! Implements hierarchical summarization for context management.
//! Compresses older messages into a rolling summary while keeping
//! recent exchanges verbatim.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::timeout;

use crate::api::chat::UIPart;
use crate::api::chats::ChatMessage;
use crate::api::token_estimation::{estimate_session_context, ContextStatus};
use crate::types::Timestamp;
use crate::error::Result;
use crate::virtues_api::client::BearerClient;

// ============================================================================
// Constants
// ============================================================================

// Note: Summarization model comes from the Lite slot (assistant_profile::get_background_model)

/// Number of recent exchanges to keep verbatim (user + assistant pairs)
/// Lower value = more aggressive compaction, but less recent context preserved
const DEFAULT_KEEP_RECENT_EXCHANGES: usize = 4;

/// Maximum tokens for summary generation
const SUMMARY_MAX_TOKENS: u32 = 1000;

/// Temperature for summary generation (lower = more deterministic)
const SUMMARY_TEMPERATURE: f32 = 0.3;

// ============================================================================
// Types
// ============================================================================

/// Result of a compaction operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    pub success: bool,
    pub messages_summarized: i32,
    pub messages_kept_verbatim: i32,
    pub summary_tokens: i64,
    pub new_usage_percentage: f64,
    pub previous_usage_percentage: f64,
    pub summary_version: i32,
}

/// Options for compaction
#[derive(Debug, Clone, Deserialize)]
pub struct CompactionOptions {
    /// Number of recent exchanges to keep verbatim (default: 4)
    #[serde(default = "default_keep_recent")]
    pub keep_recent_exchanges: usize,
    /// Force compaction even if under threshold
    #[serde(default)]
    pub force: bool,
    /// Model ID for context window lookup (uses default model if not specified)
    #[serde(default)]
    pub model_id: Option<String>,
}

fn default_keep_recent() -> usize {
    DEFAULT_KEEP_RECENT_EXCHANGES
}

impl Default for CompactionOptions {
    fn default() -> Self {
        Self {
            keep_recent_exchanges: DEFAULT_KEEP_RECENT_EXCHANGES,
            force: false,
            model_id: None,
        }
    }
}

// ============================================================================
// Summary Generation
// ============================================================================

/// System prompt for summary generation - outputs structured XML
const SUMMARY_SYSTEM_PROMPT: &str = r#"You are a conversation summarizer creating a checkpoint for conversation continuity.

Output your summary in this EXACT XML structure:

<context>
<!-- What the user is trying to accomplish - their primary goals -->
- [Goal 1]
- [Goal 2]
</context>

<decisions>
<!-- All technical decisions, preferences, and constraints established -->
- [Decision 1]
- [Decision 2]
</decisions>

<files>
<!-- Files, paths, and code locations discussed or modified -->
- [file/path 1]
- [file/path 2]
</files>

<progress>
<!-- What has been accomplished so far -->
- [Completed item 1]
- [Completed item 2]
</progress>

<pending>
<!-- Unresolved questions, pending tasks, or next steps -->
- [Pending item 1]
- [Pending item 2]
</pending>

<errors>
<!-- Key errors encountered and how they were resolved (if any) -->
- [Error and resolution]
</errors>

RULES:
- Keep total summary under 800 tokens
- Be factual and specific with concrete details (names, paths, values)
- Include ALL relevant technical details - this is a handoff document
- Omit empty sections (e.g., if no errors, omit <errors>)
- Do NOT include pleasantries, meta-commentary, or step-by-step recreation
- This summary will be injected as prior context for a new conversation instance"#;

/// Generate a summary of messages using the LLM (device bearer → virtues-api).
async fn generate_summary(
    pool: &PgPool,
    messages: &[ChatMessage],
    existing_summary: Option<&str>,
    model: &str,
) -> Result<String> {
    // Build the content to summarize
    let mut content = String::new();

    // Include existing summary if present
    if let Some(summary) = existing_summary {
        content.push_str("=== PREVIOUS SUMMARY ===\n");
        content.push_str(summary);
        content.push_str("\n\n=== NEW MESSAGES TO INCORPORATE ===\n");
    }

    // Format messages for summarization
    for msg in messages {
        let role_label = match msg.role.as_str() {
            "user" => "User",
            "assistant" => "Assistant",
            "system" => "System",
            _ => &msg.role,
        };

        content.push_str(&format!("{}: {}\n\n", role_label, msg.content));

        // Include tool calls if present
        if let Some(tool_calls) = &msg.tool_calls {
            for tc in tool_calls {
                content.push_str(&format!("  [Tool: {}]\n", tc.tool_name));
            }
        }
    }

    // Generate summary via the device bearer (auto-renews monthly).
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": SUMMARY_SYSTEM_PROMPT },
            { "role": "user", "content": content },
        ],
        "max_tokens": SUMMARY_MAX_TOKENS,
        "temperature": SUMMARY_TEMPERATURE,
    });

    // System purpose — chat compaction is automated background work.
    let response = BearerClient::from_env(pool.clone())
        .with_purpose(crate::virtues_api::client::Purpose::System)
        .with_feature("compaction")
        .post_json("/v1/ai/chat/completions", &body)
        .await
        .map_err(|e| crate::Error::Other(format!("Summary generation failed: {}", e)))?;

    if !response.is_success() {
        return Err(crate::Error::Other(format!(
            "Summary generation failed ({}): {}",
            response.status, response.body
        )));
    }

    response.body["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| crate::Error::Other("Summary response missing content".to_string()))
}

// ============================================================================
// Compaction Logic
// ============================================================================

/// Compact a chat by summarizing older messages
///
/// This function:
/// 1. Loads the chat and its messages
/// 2. Determines which messages to summarize (all except recent N exchanges)
/// 3. Generates a summary incorporating the existing summary (if any)
/// 4. Updates the chat with the new summary and metadata
pub async fn compact_chat(
    pool: &PgPool,
    chat_id: String,
    options: CompactionOptions,
) -> Result<CompactionResult> {
    let chat_id_str = chat_id.clone();

    // Load chat metadata
    let chat_row = sqlx::query(
        r#"
        SELECT
            message_count,
            conversation_summary, summary_up_to_index, summary_version
        FROM app_chats
        WHERE id = $1
        "#,
    )
    .bind(&chat_id_str)
    .fetch_optional(pool)
    .await?;

    let chat_row =
        chat_row.ok_or_else(|| crate::Error::NotFound("Chat not found".into()))?;

    use sqlx::Row;
    let _message_count: i64 = chat_row.get("message_count");
    let conversation_summary: Option<String> = chat_row.get("conversation_summary");
    let summary_up_to_index: i32 = chat_row.get::<i64, _>("summary_up_to_index") as i32;
    let summary_version: i32 = chat_row.get::<i64, _>("summary_version") as i32;

    // Load messages from normalized table
    let message_rows = sqlx::query(
        r#"
        SELECT
            id, role, content, created_at as timestamp,
            model, provider, agent_id, reasoning, tool_calls, intent, subject, thought_signature
        FROM app_chat_messages
        WHERE chat_id = $1
        ORDER BY sequence_num ASC
        "#,
    )
    .bind(&chat_id_str)
    .fetch_all(pool)
    .await?;
    
    let messages: Vec<ChatMessage> = message_rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            let id: String = row.get("id");
            let role: String = row.get("role");
            let content: String = row.get("content");
            let timestamp: Timestamp = row.get("timestamp");
            let model: Option<String> = row.get("model");
            let provider: Option<String> = row.get("provider");
            let agent_id: Option<String> = row.get("agent_id");
            let reasoning: Option<String> = row.get("reasoning");
            let tool_calls_raw: Option<serde_json::Value> = row.get("tool_calls");
            let intent_raw: Option<serde_json::Value> = row.get("intent");
            let subject: Option<String> = row.get("subject");
            let thought_signature: Option<String> = row.get("thought_signature");

            let tool_calls = tool_calls_raw
                .and_then(|tc| serde_json::from_value(tc).ok());
            let intent = intent_raw
                .and_then(|i| serde_json::from_value(i).ok());

            ChatMessage {
                id: Some(id),
                role,
                content,
                timestamp,
                model,
                provider,
                agent_id,
                reasoning,
                tool_calls,
                intent,
                subject,
                thought_signature,
                parts: None,
            }
        })
        .collect();
    
    let message_count = messages.len();

    // Calculate how many messages to keep verbatim
    // Each "exchange" is typically 2 messages (user + assistant)
    let messages_to_keep = options.keep_recent_exchanges * 2;

    // If we don't have enough messages to summarize, skip
    if message_count <= messages_to_keep && !options.force {
        return Ok(CompactionResult {
            success: true,
            messages_summarized: 0,
            messages_kept_verbatim: message_count as i32,
            summary_tokens: 0,
            new_usage_percentage: 0.0,
            previous_usage_percentage: 0.0,
            summary_version,
        });
    }

    // Determine the split point
    let split_index = if message_count > messages_to_keep {
        message_count - messages_to_keep
    } else {
        0
    };

    // Get the current summary index (messages already summarized)
    let current_summary_index = summary_up_to_index as usize;

    // Messages to add to summary (from current_summary_index to split_index)
    let messages_to_summarize = if split_index > current_summary_index {
        &messages[current_summary_index..split_index]
    } else {
        // Nothing new to summarize
        return Ok(CompactionResult {
            success: true,
            messages_summarized: current_summary_index as i32,
            messages_kept_verbatim: (message_count - current_summary_index) as i32,
            summary_tokens: 0,
            new_usage_percentage: 0.0,
            previous_usage_percentage: 0.0,
            summary_version,
        });
    };

    // Get background model from assistant profile
    let background_model = crate::api::assistant_profile::get_background_model(pool).await?;

    // Generate new summary with timeout to prevent hanging
    let new_summary = timeout(
        Duration::from_secs(60),
        generate_summary(
            pool,
            messages_to_summarize,
            conversation_summary.as_deref(),
            &background_model,
        ),
    )
    .await
    .map_err(|_| crate::Error::Other("Summary generation timed out after 60s".to_string()))??;

    // Calculate previous and new usage percentages
    // Look up context window from model registry, fallback to conservative default
    let context_window = if let Some(model_id) = &options.model_id {
        match crate::api::models::get_model(model_id).await {
            Ok(model_info) => model_info.context_window.unwrap_or(200_000) as i64,
            Err(_) => 200_000, // Conservative default if model not found
        }
    } else {
        200_000 // Conservative default if no model specified
    };
    let previous_estimate = estimate_session_context(&messages, None, None, context_window);
    let verbatim_messages = &messages[split_index..];
    let new_estimate =
        estimate_session_context(verbatim_messages, Some(&new_summary), None, context_window);

    // Update the chat metadata
    let now = Timestamp::now();
    let new_version = summary_version + 1;
    let new_summary_index = split_index as i32;

    sqlx::query(
        r#"
        UPDATE app_chats
        SET
            conversation_summary = $1,
            summary_up_to_index = $2,
            summary_version = $3,
            last_compacted_at = $4,
            updated_at = $5
        WHERE id = $6
        "#,
    )
    .bind(&new_summary)
    .bind(new_summary_index)
    .bind(new_version)
    .bind(&now)
    .bind(&now)
    .bind(&chat_id_str)
    .execute(pool)
    .await?;

    // Insert checkpoint message into chat_messages
    // This makes the checkpoint visible in the chat UI and queryable
    tracing::info!(chat_id = %chat_id, version = new_version, "Creating checkpoint message");
    let checkpoint_part = UIPart::Checkpoint {
        version: new_version,
        messages_summarized: new_summary_index,
        summary: new_summary.clone(),
        timestamp: now.to_rfc3339(),
    };

    let checkpoint_message = ChatMessage {
        id: None, // Will be generated
        role: "checkpoint".to_string(),
        content: format!("Checkpoint v{}: {} messages summarized", new_version, new_summary_index),
        timestamp: now,
        model: None,
        provider: None,
        agent_id: None,
        reasoning: None,
        tool_calls: None,
        intent: None,
        subject: None,
        thought_signature: None,
        parts: Some(vec![checkpoint_part]),
    };

    // Append the checkpoint message
    tracing::info!(chat_id = %chat_id, "Inserting checkpoint message");
    let checkpoint_msg_id = crate::api::chats::append_message(pool, chat_id.clone(), checkpoint_message).await?;
    tracing::info!(chat_id = %chat_id, msg_id = %checkpoint_msg_id, "Checkpoint message inserted");

    Ok(CompactionResult {
        success: true,
        messages_summarized: new_summary_index,
        messages_kept_verbatim: (message_count - split_index) as i32,
        summary_tokens: new_estimate.total_tokens,
        new_usage_percentage: new_estimate.usage_percentage,
        previous_usage_percentage: previous_estimate.usage_percentage,
        summary_version: new_version as i32,
    })
}

/// Build the context to send to the LLM, using checkpoint-based or legacy summary approach
///
/// This function now supports checkpoint messages:
/// 1. Finds the latest checkpoint message in the messages array
/// 2. Extracts the summary from that checkpoint
/// 3. Includes only messages AFTER the checkpoint
/// 4. Skips checkpoint messages in output (they're metadata, not conversation)
///
/// Falls back to legacy summary/summary_up_to_index if no checkpoint found.
///
/// Returns a vector of messages in OpenAI format ready for the API.
/// Note: Summary is combined into the system prompt to avoid multiple system messages,
/// which most LLM providers don't handle well.
pub fn build_context_for_llm(
    messages: &[ChatMessage],
    summary: Option<&str>,
    summary_up_to_index: usize,
    system_prompt: Option<&str>,
) -> Vec<serde_json::Value> {
    let mut context = Vec::new();

    // Find the latest checkpoint message and its index
    let (checkpoint_summary, checkpoint_index) = find_latest_checkpoint(messages);

    // Determine which summary to use (checkpoint takes precedence over legacy)
    let effective_summary = checkpoint_summary.as_deref().or(summary);
    let effective_start_index = if checkpoint_summary.is_some() {
        checkpoint_index + 1 // Start after the checkpoint message
    } else {
        summary_up_to_index
    };

    // 1. Build combined system content (prompt + summary in one message)
    // Most LLM providers only properly handle one system message
    let mut system_content = String::new();
    if let Some(prompt) = system_prompt {
        system_content.push_str(prompt);
    }

    // Append summary as part of system prompt, not separate message
    if let Some(summary_text) = effective_summary {
        if !system_content.is_empty() {
            system_content.push_str("\n\n");
        }
        system_content.push_str("<compacted_conversation>\n");
        system_content.push_str(summary_text);
        system_content.push_str("\n</compacted_conversation>");
    }

    // Only add system message if there's content
    if !system_content.is_empty() {
        context.push(serde_json::json!({
            "role": "system",
            "content": system_content
        }));
    }

    // 2. Recent messages (after checkpoint or summary_up_to_index)
    let recent_messages = if effective_start_index < messages.len() {
        &messages[effective_start_index..]
    } else {
        messages
    };

    for msg in recent_messages {
        // Skip checkpoint messages - they're metadata, not conversation
        if msg.role == "checkpoint" {
            continue;
        }

        let mut parts = Vec::new();

        // Handle parts if present
        if let Some(msg_parts) = &msg.parts {
            for part in msg_parts {
                match part {
                    UIPart::Text { text } => {
                        // An empty text block is a hard 400 from Anthropic
                        // ("text content blocks must be non-empty") and a
                        // no-op everywhere else. The composer used to send one
                        // beside every attachment-only message (the AI SDK
                        // appends a text part for text `""`), and since user
                        // parts persist verbatim, that one turn poisoned every
                        // later Claude turn in the chat while Grok kept
                        // working. Dropping it here heals chats already
                        // carrying one.
                        if text.trim().is_empty() {
                            continue;
                        }
                        parts.push(serde_json::json!({
                            "type": "text",
                            "text": text
                        }));
                    }
                    UIPart::Reasoning { text } => {
                        // Some providers support reasoning as a part
                        parts.push(serde_json::json!({
                            "type": "reasoning",
                            "reasoning": text
                        }));
                    }
                    UIPart::ToolInvocation { tool_call_id, tool_name, input, output, .. } |
                    UIPart::ToolWebSearch { tool_call_id, tool_name, input, output, .. } => {
                        // For tool invocations in history, we send them as tool calls + results
                        // This matches the OpenAI/Anthropic format for tool history
                        parts.push(serde_json::json!({
                            "type": "tool_call",
                            "id": tool_call_id,
                            "name": tool_name,
                            "arguments": input
                        }));

                        if let Some(res) = output {
                            parts.push(serde_json::json!({
                                "type": "tool_result",
                                "tool_call_id": tool_call_id,
                                "content": res
                            }));
                        }
                    }
                    UIPart::File { media_type, url, filename } => {
                        // Convert attachments to OpenAI-compatible content blocks
                        // (the gateway translates these per provider).
                        if media_type.starts_with("image/") {
                            parts.push(serde_json::json!({
                                "type": "image_url",
                                "image_url": { "url": url }
                            }));
                        } else if media_type == "application/pdf" {
                            // Vercel AI Gateway expects RAW base64 in `data` (no
                            // data: prefix) + `media_type` + `filename`.
                            let data = url.split_once(',').map(|(_, b)| b).unwrap_or(url.as_str());
                            parts.push(serde_json::json!({
                                "type": "file",
                                "file": {
                                    "data": data,
                                    "media_type": media_type,
                                    "filename": filename.clone().unwrap_or_else(|| "document.pdf".to_string())
                                }
                            }));
                        } else if media_type.starts_with("audio/") {
                            // input_audio wants raw base64 (no data: prefix) + a format token.
                            let data = url.split_once(',').map(|(_, b)| b).unwrap_or(url.as_str());
                            let format = match media_type.as_str() {
                                "audio/wav" | "audio/x-wav" => "wav",
                                "audio/ogg" => "ogg",
                                "audio/flac" => "flac",
                                "audio/aac" | "audio/x-m4a" | "audio/mp4" => "m4a",
                                "audio/webm" => "webm",
                                _ => "mp3",
                            };
                            parts.push(serde_json::json!({
                                "type": "input_audio",
                                "input_audio": { "data": data, "format": format }
                            }));
                        } else if media_type.starts_with("text/") || media_type == "application/json" {
                            // Text/code/doc files (and long pasted text) ride inline as
                            // a text block so any model can read them. The data URL holds
                            // base64 UTF-8.
                            let b64 = url.split_once(',').map(|(_, b)| b).unwrap_or(url.as_str());
                            let content = base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                b64,
                            )
                            .ok()
                            .and_then(|bytes| String::from_utf8(bytes).ok())
                            .unwrap_or_default();
                            let name = filename.clone().unwrap_or_else(|| "file.txt".to_string());
                            parts.push(serde_json::json!({
                                "type": "text",
                                "text": format!("[File: {}]\n{}", name, content)
                            }));
                        } else {
                            // Unknown type — at least make the model aware of it.
                            parts.push(serde_json::json!({
                                "type": "text",
                                "text": format!("[Attached file: {}]", filename.clone().unwrap_or_else(|| media_type.clone()))
                            }));
                        }
                    }
                    UIPart::Checkpoint { .. } => {
                        // Skip checkpoint parts - handled above
                    }
                    UIPart::Unknown => {}
                }
            }
        }

        // If no parts (legacy), use content
        let content = if parts.is_empty() {
            // Nothing survived and there is no legacy content either: this
            // message would go out as `"content": ""`, which is the same
            // rejection in string form. Leaving it out is the only honest
            // rendering of a message with nothing in it.
            if msg.content.trim().is_empty() {
                continue;
            }
            serde_json::Value::String(msg.content.clone())
        } else {
            serde_json::Value::Array(parts)
        };

        context.push(serde_json::json!({
            "role": msg.role,
            "content": content
        }));
    }

    context
}

/// Find the latest checkpoint message and extract its summary
///
/// Returns (Option<summary_text>, checkpoint_index)
/// If no checkpoint found, returns (None, 0)
fn find_latest_checkpoint(messages: &[ChatMessage]) -> (Option<String>, usize) {
    // Search from the end to find the most recent checkpoint
    for (idx, msg) in messages.iter().enumerate().rev() {
        if msg.role == "checkpoint" {
            // Extract summary from checkpoint part
            if let Some(parts) = &msg.parts {
                for part in parts {
                    if let UIPart::Checkpoint { summary, .. } = part {
                        return (Some(summary.clone()), idx);
                    }
                }
            }
        }
    }
    (None, 0)
}

/// Check if a chat needs compaction based on context usage
pub async fn needs_compaction(
    pool: &PgPool,
    chat_id: String,
    context_window: i64,
) -> Result<ContextStatus> {
    let chat_id_str = chat_id.clone();

    // Load chat metadata
    let chat_row = sqlx::query(
        r#"
        SELECT conversation_summary, summary_up_to_index
        FROM app_chats
        WHERE id = $1
        "#,
    )
    .bind(&chat_id_str)
    .fetch_optional(pool)
    .await?;

    let chat_row =
        chat_row.ok_or_else(|| crate::Error::NotFound("Chat not found".into()))?;

    use sqlx::Row;
    let conversation_summary: Option<String> = chat_row.get("conversation_summary");
    let summary_up_to_index: i32 = chat_row.get::<i64, _>("summary_up_to_index") as i32;

    // Load messages from normalized table
    let message_rows = sqlx::query(
        r#"
        SELECT
            id, role, content, created_at as timestamp,
            model, provider, agent_id, reasoning, tool_calls, intent, subject, thought_signature
        FROM app_chat_messages
        WHERE chat_id = $1
        ORDER BY sequence_num ASC
        "#,
    )
    .bind(&chat_id_str)
    .fetch_all(pool)
    .await?;

    let messages: Vec<ChatMessage> = message_rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            let id: String = row.get("id");
            let role: String = row.get("role");
            let content: String = row.get("content");
            let timestamp: Timestamp = row.get("timestamp");
            let model: Option<String> = row.get("model");
            let provider: Option<String> = row.get("provider");
            let agent_id: Option<String> = row.get("agent_id");
            let reasoning: Option<String> = row.get("reasoning");
            let tool_calls_raw: Option<serde_json::Value> = row.get("tool_calls");
            let intent_raw: Option<serde_json::Value> = row.get("intent");
            let subject: Option<String> = row.get("subject");
            let thought_signature: Option<String> = row.get("thought_signature");

            let tool_calls = tool_calls_raw
                .and_then(|tc| serde_json::from_value(tc).ok());
            let intent = intent_raw
                .and_then(|i| serde_json::from_value(i).ok());

            ChatMessage {
                id: Some(id),
                role,
                content,
                timestamp,
                model,
                provider,
                agent_id,
                reasoning,
                tool_calls,
                intent,
                subject,
                thought_signature,
                parts: None,
            }
        })
        .collect();

    // Get verbatim messages (after summary)
    let verbatim_messages = if (summary_up_to_index as usize) < messages.len() {
        &messages[(summary_up_to_index as usize)..]
    } else {
        &messages[..]
    };

    // Estimate context with summary + verbatim messages
    let estimate = estimate_session_context(
        verbatim_messages,
        conversation_summary.as_deref(),
        None,
        context_window,
    );

    Ok(estimate.status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_context_without_summary() {
        let messages = vec![
            ChatMessage {
                id: None,
                role: "user".to_string(),
                content: "Hello".to_string(),
                timestamp: Timestamp::parse("2024-01-01T00:00:00Z").unwrap(),
                model: None,
                provider: None,
                agent_id: None,
                tool_calls: None,
                reasoning: None,
                intent: None,
                subject: None,
                thought_signature: None,
                parts: None,
            },
            ChatMessage {
                id: None,
                role: "assistant".to_string(),
                content: "Hi there!".to_string(),
                timestamp: Timestamp::parse("2024-01-01T00:00:01Z").unwrap(),
                model: None,
                provider: None,
                agent_id: None,
                tool_calls: None,
                reasoning: None,
                intent: None,
                subject: None,
                thought_signature: None,
                parts: None,
            },
        ];

        let context = build_context_for_llm(&messages, None, 0, Some("You are helpful."));

        assert_eq!(context.len(), 3); // system + 2 messages
        assert_eq!(context[0]["role"], "system");
        assert_eq!(context[1]["role"], "user");
        assert_eq!(context[2]["role"], "assistant");
    }

    /// An attachment-only send arrives as `[file, text ""]`. The file block
    /// must survive and the empty text block must not; a message that is
    /// empty through and through must not be sent at all.
    #[test]
    fn test_build_context_drops_empty_text_blocks() {
        let base = ChatMessage {
            id: None,
            role: "user".to_string(),
            content: String::new(),
            timestamp: Timestamp::parse("2024-01-01T00:00:00Z").unwrap(),
            model: None,
            provider: None,
            agent_id: None,
            tool_calls: None,
            reasoning: None,
            intent: None,
            subject: None,
            thought_signature: None,
            parts: None,
        };
        let messages = vec![
            ChatMessage {
                parts: Some(vec![
                    UIPart::File {
                        media_type: "image/png".to_string(),
                        url: "data:image/png;base64,AAAA".to_string(),
                        filename: Some("shot.png".to_string()),
                    },
                    UIPart::Text { text: String::new() },
                ]),
                ..base.clone()
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "I see it.".to_string(),
                ..base.clone()
            },
            // Empty in every representation: parts that all drop, no content.
            ChatMessage {
                parts: Some(vec![UIPart::Text { text: "   ".to_string() }]),
                ..base.clone()
            },
            ChatMessage {
                content: "and then?".to_string(),
                ..base
            },
        ];

        let context = build_context_for_llm(&messages, None, 0, None);

        assert_eq!(context.len(), 3, "the all-empty message is omitted");
        let first = context[0]["content"].as_array().expect("parts array");
        assert_eq!(first.len(), 1, "only the file block survives");
        assert_eq!(first[0]["type"], "image_url");
        assert_eq!(context[1]["role"], "assistant");
        assert_eq!(context[2]["content"], "and then?");
    }

    #[test]
    fn test_build_context_with_summary() {
        let messages = vec![
            ChatMessage {
                id: None,
                role: "user".to_string(),
                content: "Old message".to_string(),
                timestamp: Timestamp::parse("2024-01-01T00:00:00Z").unwrap(),
                model: None,
                provider: None,
                agent_id: None,
                tool_calls: None,
                reasoning: None,
                intent: None,
                subject: None,
                thought_signature: None,
                parts: None,
            },
            ChatMessage {
                id: None,
                role: "assistant".to_string(),
                content: "Old response".to_string(),
                timestamp: Timestamp::parse("2024-01-01T00:00:01Z").unwrap(),
                model: None,
                provider: None,
                agent_id: None,
                tool_calls: None,
                reasoning: None,
                intent: None,
                subject: None,
                thought_signature: None,
                parts: None,
            },
            ChatMessage {
                id: None,
                role: "user".to_string(),
                content: "Recent message".to_string(),
                timestamp: Timestamp::parse("2024-01-01T00:00:02Z").unwrap(),
                model: None,
                provider: None,
                agent_id: None,
                tool_calls: None,
                reasoning: None,
                intent: None,
                subject: None,
                thought_signature: None,
                parts: None,
            },
        ];

        // summary_up_to_index = 2 means first 2 messages are summarized
        let context = build_context_for_llm(
            &messages,
            Some("User asked about something."),
            2,
            Some("You are helpful."),
        );

        // Should have: combined system prompt (with summary), 1 recent message
        assert_eq!(context.len(), 2);
        // System message should contain both prompt and summary
        let system_content = context[0]["content"].as_str().unwrap();
        assert!(system_content.contains("You are helpful."));
        assert!(system_content.contains("<compacted_conversation>"));
        assert!(system_content.contains("User asked about something."));
        assert_eq!(context[1]["content"], "Recent message");
    }
}
