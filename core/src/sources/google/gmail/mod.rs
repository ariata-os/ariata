//! Google Gmail stream implementation

pub mod transform;

use async_trait::async_trait;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    client::GoogleClient,
    config::{GmailSyncMode, GoogleGmailConfig},
    types::{
        HistoryResponse, Message, MessagePart, MessagesListResponse, Thread, ThreadsListResponse,
    },
};
use crate::{
    error::Result,
    sources::{
        auth::SourceAuth,
        base::{ConfigSerializable, SyncMode, SyncResult},
        pull_stream::PullStream,
    },
    storage::stream_writer::StreamWriter,
};

/// Google Gmail stream
///
/// Syncs email messages from Gmail API to object storage via StreamWriter.
pub struct GoogleGmailStream {
    source_id: Uuid,
    client: GoogleClient,
    db: PgPool,
    stream_writer: Arc<Mutex<StreamWriter>>,
    config: GoogleGmailConfig,
}

impl GoogleGmailStream {
    /// Create a new Gmail stream with SourceAuth and StreamWriter
    pub fn new(
        source_id: Uuid,
        db: PgPool,
        stream_writer: Arc<Mutex<StreamWriter>>,
        auth: SourceAuth,
    ) -> Self {
        // Extract token manager from auth
        let token_manager = auth
            .token_manager()
            .expect("GoogleGmailStream requires OAuth2 auth")
            .clone();

        let client = GoogleClient::with_api(source_id, token_manager, "gmail", "v1");

        Self {
            source_id,
            client,
            db,
            stream_writer,
            config: GoogleGmailConfig::default(),
        }
    }

    /// Load configuration from database (called by PullStream trait)
    async fn load_config_internal(&mut self, db: &PgPool, source_id: Uuid) -> Result<()> {
        // Try loading from stream_connections table first (new pattern)
        let result = sqlx::query_as::<_, (serde_json::Value,)>(
            "SELECT config FROM stream_connections WHERE source_connection_id = $1 AND stream_name = 'gmail'",
        )
        .bind(source_id)
        .fetch_optional(db)
        .await?;

        if let Some((config_json,)) = result {
            if let Ok(config) = GoogleGmailConfig::from_json(&config_json) {
                self.config = config;
            }
        }

        Ok(())
    }

    /// Sync Gmail messages with explicit sync mode
    #[tracing::instrument(skip(self), fields(source_id = %self.source_id, mode = ?sync_mode))]
    pub async fn sync_with_mode(&self, sync_mode: &SyncMode) -> Result<SyncResult> {
        tracing::info!("Starting Gmail sync");

        // Execute the sync (logging is handled by job executor)
        self.sync_internal(&sync_mode).await
    }

    /// Internal sync implementation
    #[tracing::instrument(skip(self), fields(source_id = %self.source_id, mode = ?sync_mode))]
    async fn sync_internal(&self, sync_mode: &SyncMode) -> Result<SyncResult> {
        let started_at = Utc::now();
        let records_fetched;
        let records_written;
        let records_failed;
        let next_cursor;

        // Load last sync token from database as defensive fallback
        let db_history_id = self.get_last_sync_token().await?;

        // Determine effective cursor: prefer SyncMode parameter, fall back to database
        let effective_cursor = match sync_mode {
            SyncMode::Incremental { cursor } => cursor.clone().or(db_history_id),
            SyncMode::FullRefresh => None,
        };

        match effective_cursor {
            Some(ref history_id) => {
                // Use history API for incremental sync
                let result = self.sync_incremental(history_id).await?;

                records_fetched = result.0;
                records_written = result.1;
                records_failed = result.2;
                next_cursor = result.3;
            }
            None => {
                // Full sync - fetch messages based on config
                match self.config.sync_mode {
                    GmailSyncMode::Messages => {
                        let result = self.sync_messages_full().await?;
                        records_fetched = result.0;
                        records_written = result.1;
                        records_failed = result.2;
                        next_cursor = result.3;
                    }
                    GmailSyncMode::Threads => {
                        let result = self.sync_threads_full().await?;
                        records_fetched = result.0;
                        records_written = result.1;
                        records_failed = result.2;
                        next_cursor = result.3;
                    }
                }
            }
        }

        let completed_at = Utc::now();

        // Collect records from StreamWriter for archive and transform pipeline
        let records = {
            let mut writer = self.stream_writer.lock().await;
            let collected = writer
                .collect_records(self.source_id, "gmail")
                .map(|(records, _, _)| records);

            if let Some(ref recs) = collected {
                tracing::info!(
                    record_count = recs.len(),
                    "Collected Gmail records from StreamWriter"
                );
            } else {
                tracing::warn!("No Gmail records collected from StreamWriter");
            }

            collected
        };

        Ok(SyncResult {
            records_fetched,
            records_written,
            records_failed,
            next_cursor,
            started_at,
            completed_at,
            records, // Return collected records for archive/transform
            archive_job_id: None,
        })
    }

    /// Sync using history API (incremental)
    async fn sync_incremental(
        &self,
        history_id: &str,
    ) -> Result<(usize, usize, usize, Option<String>)> {
        let mut records_fetched = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut latest_history_id = history_id.to_string();

        let params = vec![("startHistoryId", history_id)];

        let response: HistoryResponse = self
            .client
            .get_with_params("users/me/history", &params)
            .await?;

        if let Some(history) = response.history {
            for record in history {
                // Process messages added
                if let Some(messages_added) = record.messages_added {
                    for item in messages_added {
                        records_fetched += 1;

                        // Fetch full message
                        match self.fetch_and_store_message(&item.message.id).await {
                            Ok(true) => records_written += 1,
                            Ok(false) => records_failed += 1,
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to fetch message {}", item.message.id);
                                records_failed += 1;
                            }
                        }
                    }
                }
            }
        }

        // Update history ID
        if let Some(new_history_id) = response.history_id {
            latest_history_id = new_history_id;
            self.save_history_id(&latest_history_id).await?;
        }

        Ok((
            records_fetched,
            records_written,
            records_failed,
            Some(latest_history_id),
        ))
    }

    /// Full sync of messages with pagination
    async fn sync_messages_full(&self) -> Result<(usize, usize, usize, Option<String>)> {
        let mut records_fetched = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut latest_history_id = None;
        let mut page_token: Option<String> = None;

        loop {
            // Build query parameters
            let mut params = vec![("maxResults", self.config.max_messages_per_sync.to_string())];

            // Add label filters
            for label in &self.config.label_ids {
                params.push(("labelIds", label.clone()));
            }

            // Add query
            let query = self.config.build_query();
            if !query.is_empty() {
                params.push(("q", query));
            }

            if self.config.include_spam_trash {
                params.push(("includeSpamTrash", "true".to_string()));
            }

            // Add page token if we have one
            if let Some(ref token) = page_token {
                params.push(("pageToken", token.clone()));
            }

            let param_refs: Vec<(&str, &str)> =
                params.iter().map(|(k, v)| (*k, v.as_str())).collect();

            // List messages
            let response: MessagesListResponse = self
                .client
                .get_with_params("users/me/messages", &param_refs)
                .await?;

            if let Some(messages) = response.messages {
                for msg_ref in messages {
                    records_fetched += 1;

                    // Fetch full message
                    match self.fetch_and_store_message(&msg_ref.id).await {
                        Ok(true) => records_written += 1,
                        Ok(false) => records_failed += 1,
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to fetch message {}", msg_ref.id);
                            records_failed += 1;
                        }
                    }
                }
            }

            // Check if there are more pages
            page_token = response.next_page_token;
            if page_token.is_none() {
                break;
            }

            // Only log every 5th page or the last page
            if records_fetched % 250 == 0 || page_token.is_none() {
                tracing::debug!(
                    messages_fetched = records_fetched,
                    has_more = page_token.is_some(),
                    "Gmail sync progress"
                );
            }
        }

        tracing::info!(
            total_messages = records_fetched,
            written = records_written,
            failed = records_failed,
            "Completed paginated messages sync"
        );

        // Get profile to fetch latest history ID for future incremental syncs
        if let Ok(profile) = self.get_profile().await {
            if let Some(history_id) = profile.get("historyId").and_then(|v| v.as_str()) {
                latest_history_id = Some(history_id.to_string());
                self.save_history_id(history_id).await?;
            }
        }

        Ok((
            records_fetched,
            records_written,
            records_failed,
            latest_history_id,
        ))
    }

    /// Full sync of threads with pagination
    async fn sync_threads_full(&self) -> Result<(usize, usize, usize, Option<String>)> {
        let mut records_fetched = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut latest_history_id = None;
        let mut page_token: Option<String> = None;

        loop {
            // Build query parameters
            let mut params = vec![("maxResults", self.config.max_messages_per_sync.to_string())];

            // Add label filters
            for label in &self.config.label_ids {
                params.push(("labelIds", label.clone()));
            }

            // Add query
            let query = self.config.build_query();
            if !query.is_empty() {
                params.push(("q", query));
            }

            if self.config.include_spam_trash {
                params.push(("includeSpamTrash", "true".to_string()));
            }

            // Add page token if we have one
            if let Some(ref token) = page_token {
                params.push(("pageToken", token.clone()));
            }

            let param_refs: Vec<(&str, &str)> =
                params.iter().map(|(k, v)| (*k, v.as_str())).collect();

            // List threads
            let response: ThreadsListResponse = self
                .client
                .get_with_params("users/me/threads", &param_refs)
                .await?;

            if let Some(threads) = response.threads {
                for thread_ref in threads {
                    // Fetch full thread with messages
                    let thread: Thread = self
                        .client
                        .get(&format!("users/me/threads/{}", thread_ref.id))
                        .await?;

                    if let Some(messages) = thread.messages {
                        let thread_message_count = messages.len();

                        for (position, message) in messages.into_iter().enumerate() {
                            records_fetched += 1;

                            // Store message with thread context
                            match self
                                .store_message(
                                    message,
                                    Some(position as i32 + 1),
                                    Some(thread_message_count as i32),
                                )
                                .await
                            {
                                Ok(true) => records_written += 1,
                                Ok(false) => records_failed += 1,
                                Err(e) => {
                                    tracing::warn!(error = %e, "Failed to store message from thread {}", thread_ref.id);
                                    records_failed += 1;
                                }
                            }
                        }
                    }

                    // Update history ID from thread
                    if let Some(history_id) = thread.history_id {
                        latest_history_id = Some(history_id.clone());
                    }
                }
            }

            // Check if there are more pages
            page_token = response.next_page_token;
            if page_token.is_none() {
                break;
            }

            // Only log every 5th page or the last page
            if records_fetched % 250 == 0 || page_token.is_none() {
                tracing::debug!(
                    messages_fetched = records_fetched,
                    has_more = page_token.is_some(),
                    "Gmail thread sync progress"
                );
            }
        }

        tracing::info!(
            total_messages = records_fetched,
            written = records_written,
            failed = records_failed,
            "Completed paginated threads sync"
        );

        // Save latest history ID for incremental sync
        if let Some(ref history_id) = latest_history_id {
            self.save_history_id(history_id).await?;
        }

        Ok((
            records_fetched,
            records_written,
            records_failed,
            latest_history_id,
        ))
    }

    /// Fetch a single message and store it
    async fn fetch_and_store_message(&self, message_id: &str) -> Result<bool> {
        let message: Message = self
            .client
            .get(&format!("users/me/messages/{message_id}"))
            .await?;
        self.store_message(message, None, None).await
    }

    /// Store a message in the database
    async fn store_message(
        &self,
        message: Message,
        thread_position: Option<i32>,
        thread_message_count: Option<i32>,
    ) -> Result<bool> {
        // Extract headers into a map
        let mut headers_map = HashMap::new();
        if let Some(ref payload) = message.payload {
            if let Some(ref headers) = payload.headers {
                for header in headers {
                    headers_map.insert(header.name.clone(), header.value.clone());
                }
            }
        }

        // Extract key fields from headers
        let subject = headers_map.get("Subject").cloned();
        let from = headers_map.get("From").cloned();
        let to = headers_map.get("To").cloned();
        let cc = headers_map.get("Cc").cloned();
        let bcc = headers_map.get("Bcc").cloned();
        let reply_to = headers_map.get("Reply-To").cloned();
        let date_str = headers_map.get("Date").cloned();

        // Parse email addresses
        let (from_email, from_name) = self.parse_email_address(from.as_deref());
        let (to_emails, to_names) = self.parse_email_list(to.as_deref());
        let (cc_emails, cc_names) = self.parse_email_list(cc.as_deref());
        let (bcc_emails, bcc_names) = self.parse_email_list(bcc.as_deref());

        // Parse date
        let date = if let Some(date_str) = date_str {
            self.parse_email_date(&date_str).unwrap_or_else(Utc::now)
        } else {
            Utc::now()
        };

        // Parse internal date (milliseconds since epoch)
        let internal_date = message
            .internal_date
            .as_ref()
            .and_then(|ms_str| ms_str.parse::<i64>().ok())
            .and_then(DateTime::from_timestamp_millis);

        // Extract body content
        let (body_plain, body_html, attachments) = if self.config.fetch_body {
            self.extract_message_content(&message.payload)
        } else {
            (None, None, Vec::new())
        };

        // Process attachments
        let has_attachments = !attachments.is_empty();
        let attachment_count = attachments.len() as i32;
        let attachment_types: Vec<String> = attachments.iter().map(|a| a.0.clone()).collect();
        let attachment_names: Vec<String> = attachments.iter().map(|a| a.1.clone()).collect();
        let attachment_sizes: Vec<i32> = attachments.iter().map(|a| a.2).collect();

        // Process labels
        let labels = message.label_ids.clone().unwrap_or_default();
        let is_unread = labels.contains(&"UNREAD".to_string());
        let is_important = labels.contains(&"IMPORTANT".to_string());
        let is_starred = labels.contains(&"STARRED".to_string());
        let is_draft = labels.contains(&"DRAFT".to_string());
        let is_sent = labels.contains(&"SENT".to_string());
        let is_trash = labels.contains(&"TRASH".to_string());
        let is_spam = labels.contains(&"SPAM".to_string());

        // Build complete record with all parsed fields for storage
        let record = serde_json::json!({
            "message_id": message.id,
            "thread_id": message.thread_id,
            "history_id": message.history_id,
            "subject": subject,
            "snippet": message.snippet,
            "date": date,
            "from_email": from_email,
            "from_name": from_name,
            "to_emails": to_emails,
            "to_names": to_names,
            "cc_emails": cc_emails,
            "cc_names": cc_names,
            "bcc_emails": bcc_emails,
            "bcc_names": bcc_names,
            "reply_to": reply_to,
            "body_plain": body_plain,
            "body_html": body_html,
            "has_attachments": has_attachments,
            "attachment_count": attachment_count,
            "attachment_types": attachment_types,
            "attachment_names": attachment_names,
            "attachment_sizes_bytes": attachment_sizes,
            "labels": labels,
            "is_unread": is_unread,
            "is_important": is_important,
            "is_starred": is_starred,
            "is_draft": is_draft,
            "is_sent": is_sent,
            "is_trash": is_trash,
            "is_spam": is_spam,
            "thread_position": thread_position,
            "thread_message_count": thread_message_count,
            "size_bytes": message.size_estimate,
            "internal_date": internal_date,
            "raw_message": message,
            "headers": headers_map,
            "synced_at": Utc::now(),
        });

        // Write to S3/object storage via StreamWriter
        {
            let mut writer = self.stream_writer.lock().await;
            writer.write_record(self.source_id, "gmail", record, Some(date))?;
        }

        tracing::trace!(message_id = %message.id, "Wrote Gmail message to object storage");
        Ok(true)
    }

    /// Extract plain text, HTML, and attachments from message payload
    fn extract_message_content(
        &self,
        payload: &Option<MessagePart>,
    ) -> (Option<String>, Option<String>, Vec<(String, String, i32)>) {
        let mut plain_text = None;
        let mut html_text = None;
        let mut attachments = Vec::new();

        if let Some(part) = payload {
            self.extract_from_part(part, &mut plain_text, &mut html_text, &mut attachments);
        }

        (plain_text, html_text, attachments)
    }

    /// Recursively extract content from message parts
    fn extract_from_part(
        &self,
        part: &MessagePart,
        plain_text: &mut Option<String>,
        html_text: &mut Option<String>,
        attachments: &mut Vec<(String, String, i32)>,
    ) {
        // Check if this is an attachment
        if let Some(filename) = &part.filename {
            if !filename.is_empty() {
                let mime_type = part
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                let size = part.body.as_ref().map(|b| b.size).unwrap_or(0);
                attachments.push((mime_type, filename.clone(), size));
                return;
            }
        }

        // Extract text content
        if let Some(mime_type) = &part.mime_type {
            if mime_type == "text/plain" && plain_text.is_none() {
                if let Some(body) = &part.body {
                    if let Some(data) = &body.data {
                        // Decode base64url
                        if let Ok(decoded) =
                            base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data)
                        {
                            *plain_text = String::from_utf8(decoded).ok();
                        }
                    }
                }
            } else if mime_type == "text/html" && html_text.is_none() {
                if let Some(body) = &part.body {
                    if let Some(data) = &body.data {
                        // Decode base64url
                        if let Ok(decoded) =
                            base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data)
                        {
                            *html_text = String::from_utf8(decoded).ok();
                        }
                    }
                }
            }
        }

        // Recursively process parts
        if let Some(parts) = &part.parts {
            for sub_part in parts {
                self.extract_from_part(sub_part, plain_text, html_text, attachments);
            }
        }
    }

    /// Parse email address into email and name components
    fn parse_email_address(&self, address: Option<&str>) -> (Option<String>, Option<String>) {
        if let Some(addr) = address {
            if let Some(start) = addr.rfind('<') {
                if let Some(end) = addr.rfind('>') {
                    let email = addr[start + 1..end].trim().to_string();
                    let name = addr[..start].trim().trim_matches('"').to_string();
                    return (Some(email), if name.is_empty() { None } else { Some(name) });
                }
            }
            // Just an email address without name
            return (Some(addr.trim().to_string()), None);
        }
        (None, None)
    }

    /// Parse comma-separated email list
    fn parse_email_list(&self, addresses: Option<&str>) -> (Vec<String>, Vec<String>) {
        let mut emails = Vec::new();
        let mut names = Vec::new();

        if let Some(addr_list) = addresses {
            for addr in addr_list.split(',') {
                let (email, name) = self.parse_email_address(Some(addr.trim()));
                if let Some(e) = email {
                    emails.push(e);
                    names.push(name.unwrap_or_default());
                }
            }
        }

        (emails, names)
    }

    /// Parse email date header
    fn parse_email_date(&self, date_str: &str) -> Option<DateTime<Utc>> {
        // Try RFC2822 format first (most common)
        if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
            return Some(dt.with_timezone(&Utc));
        }

        // Try RFC3339 as fallback
        if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
            return Some(dt.with_timezone(&Utc));
        }

        None
    }

    /// Get user profile (for history ID)
    async fn get_profile(&self) -> Result<serde_json::Value> {
        self.client.get("users/me/profile").await
    }

    /// Get the last sync token (history ID) from the database
    async fn get_last_sync_token(&self) -> Result<Option<String>> {
        let row = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT last_sync_token FROM stream_connections WHERE source_connection_id = $1 AND stream_name = 'gmail'",
        )
        .bind(self.source_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.and_then(|(token,)| token))
    }

    /// Save the history ID to the database
    async fn save_history_id(&self, history_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE data.stream_connections SET last_sync_token = $1, last_sync_at = $2 WHERE source_connection_id = $3 AND stream_name = 'gmail'",
        )
        .bind(history_id)
        .bind(Utc::now())
        .bind(self.source_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

// Implement PullStream trait for GoogleGmailStream
#[async_trait]
impl PullStream for GoogleGmailStream {
    async fn sync_pull(&self, mode: SyncMode) -> Result<SyncResult> {
        self.sync_with_mode(&mode).await
    }

    async fn load_config(&mut self, db: &PgPool, source_id: Uuid) -> Result<()> {
        self.load_config_internal(db, source_id).await
    }

    fn table_name(&self) -> &str {
        "stream_google_gmail"
    }

    fn stream_name(&self) -> &str {
        "gmail"
    }

    fn source_name(&self) -> &str {
        "google"
    }

    fn supports_incremental(&self) -> bool {
        true
    }

    fn supports_full_refresh(&self) -> bool {
        true
    }
}
