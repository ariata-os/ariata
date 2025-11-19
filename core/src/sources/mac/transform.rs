///! macOS stream transformations to ontology tables
///!
///! Transforms raw macOS device data (apps, browser, iMessage) into normalized ontology tables.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::error::Result;
use crate::jobs::transform_context::TransformContext;
use crate::sources::base::{OntologyTransform, TransformResult};

/// Batch size for database inserts
const BATCH_SIZE: usize = 500;

// ============================================================================
// MacAppsTransform: stream_mac_apps → activity_app_usage
// ============================================================================

/// Transform macOS app usage events to activity_app_usage ontology
///
/// Aggregates discrete focus events (focus_gained/focus_lost) into temporal usage sessions.
pub struct MacAppsTransform;

#[async_trait]
impl OntologyTransform for MacAppsTransform {
    fn source_table(&self) -> &str {
        "stream_mac_apps"
    }

    fn target_table(&self) -> &str {
        "activity_app_usage"
    }

    fn domain(&self) -> &str {
        "activity"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(
        &self,
        db: &Database,
        context: &TransformContext,
        source_id: Uuid,
    ) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting macOS Apps to activity_app_usage transformation"
        );

        // Read stream data from data source using checkpoint
        let checkpoint_key = "mac_apps_to_activity_app_usage";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| {
            crate::Error::Other("No data source available for transform".to_string())
        })?;
        let batches = data_source
            .read_with_checkpoint(source_id, "apps", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched macOS app event batches from data source"
        );

        // Collect all events first for aggregation
        let mut all_events: Vec<AppEvent> = Vec::new();
        let mut max_batch_timestamp: Option<DateTime<Utc>> = None;

        for batch in &batches {
            for record in &batch.records {
                records_read += 1;

                // Extract fields from JSONL record
                let timestamp = record
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok());

                let event_type = record
                    .get("event_type")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let app_name = record
                    .get("app_name")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let bundle_id = record
                    .get("bundle_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let stream_id = record
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                // Validate required fields
                if let (Some(ts), Some(et), Some(an)) = (timestamp, event_type, app_name) {
                    all_events.push(AppEvent {
                        timestamp: ts,
                        event_type: et,
                        app_name: an,
                        bundle_id,
                        stream_id,
                    });
                }
            }

            if let Some(max_ts) = batch.max_timestamp {
                max_batch_timestamp = Some(max_ts);
            }
        }

        // Sort events by timestamp for sequential processing
        all_events.sort_by_key(|e| e.timestamp);

        tracing::info!(
            total_events = all_events.len(),
            "Aggregating app events into usage sessions"
        );

        // Aggregate events into usage sessions
        let sessions = aggregate_app_events_to_sessions(all_events);

        tracing::info!(
            session_count = sessions.len(),
            "Created usage sessions from app events"
        );

        // Batch insert sessions
        let mut pending_records: Vec<(
            String,           // app_name
            Option<String>,   // app_bundle_id
            DateTime<Utc>,    // start_time
            DateTime<Utc>,    // end_time
            Uuid,             // source_stream_id
        )> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;
        let processing_start = std::time::Instant::now();

        for session in sessions {
            pending_records.push((
                session.app_name.clone(),
                session.bundle_id.clone(),
                session.start_time,
                session.end_time,
                session.stream_id,
            ));

            last_processed_id = Some(session.stream_id);

            // Execute batch insert when we reach batch size
            if pending_records.len() >= BATCH_SIZE {
                let insert_start = std::time::Instant::now();
                let batch_result = execute_app_usage_batch_insert(db, &pending_records).await;
                let insert_duration = insert_start.elapsed();
                batch_insert_total_ms += insert_duration.as_millis();
                batch_insert_count += 1;

                tracing::info!(
                    batch_size = pending_records.len(),
                    insert_duration_ms = insert_duration.as_millis(),
                    "Executed batch insert"
                );

                match batch_result {
                    Ok(written) => {
                        records_written += written;
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            batch_size = pending_records.len(),
                            "Batch insert failed"
                        );
                        records_failed += pending_records.len();
                    }
                }
                pending_records.clear();
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_app_usage_batch_insert(db, &pending_records).await;
            let insert_duration = insert_start.elapsed();
            batch_insert_total_ms += insert_duration.as_millis();
            batch_insert_count += 1;

            tracing::info!(
                batch_size = pending_records.len(),
                insert_duration_ms = insert_duration.as_millis(),
                "Executed final batch insert"
            );

            match batch_result {
                Ok(written) => {
                    records_written += written;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                    records_failed += pending_records.len();
                }
            }
        }

        // Update checkpoint after processing all batches
        if let Some(max_ts) = max_batch_timestamp {
            data_source
                .update_checkpoint(source_id, "apps", checkpoint_key, max_ts)
                .await?;
        }

        let processing_duration = processing_start.elapsed();
        let total_duration = transform_start.elapsed();

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            processing_duration_ms = processing_duration.as_millis(),
            read_duration_ms = read_duration.as_millis(),
            batch_insert_total_ms,
            batch_insert_count,
            avg_batch_insert_ms = if batch_insert_count > 0 { batch_insert_total_ms / batch_insert_count as u128 } else { 0 },
            "macOS Apps to activity_app_usage transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![],
        })
    }
}

/// App event from stream
#[derive(Debug, Clone)]
struct AppEvent {
    timestamp: DateTime<Utc>,
    event_type: String,
    app_name: String,
    bundle_id: Option<String>,
    stream_id: Uuid,
}

/// App usage session (temporal bounds)
#[derive(Debug, Clone)]
struct AppSession {
    app_name: String,
    bundle_id: Option<String>,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    stream_id: Uuid,
}

/// Aggregate discrete app events into temporal usage sessions
///
/// Strategy:
/// - Track currently focused app
/// - focus_gained: start new session
/// - focus_lost/quit: end current session
/// - Consecutive focus_gained without focus_lost: implicitly end previous session
fn aggregate_app_events_to_sessions(events: Vec<AppEvent>) -> Vec<AppSession> {
    let mut sessions = Vec::new();
    let mut current_session: Option<AppSession> = None;
    let events_count = events.len();

    for event in events {
        match event.event_type.as_str() {
            "focus_gained" | "launch" => {
                // If there's a current session, end it at this event's timestamp
                if let Some(mut session) = current_session.take() {
                    session.end_time = event.timestamp;
                    sessions.push(session);
                }

                // Start new session
                current_session = Some(AppSession {
                    app_name: event.app_name.clone(),
                    bundle_id: event.bundle_id.clone(),
                    start_time: event.timestamp,
                    end_time: event.timestamp, // Will be updated when session ends
                    stream_id: event.stream_id,
                });
            }
            "focus_lost" | "quit" => {
                // End current session
                if let Some(mut session) = current_session.take() {
                    session.end_time = event.timestamp;
                    // Only add sessions with valid duration (>= 0 seconds, allows zero-duration for quick switches)
                    if session.end_time >= session.start_time {
                        sessions.push(session);
                    } else {
                        tracing::warn!(
                            app_name = %session.app_name,
                            start = %session.start_time,
                            end = %session.end_time,
                            duration_ms = session.end_time.signed_duration_since(session.start_time).num_milliseconds(),
                            "Filtering out session with negative duration (clock skew or data error)"
                        );
                    }
                }
            }
            _ => {
                tracing::debug!(event_type = %event.event_type, "Unknown event type");
            }
        }
    }

    // Handle any unclosed session (set end_time to start_time + 1 minute as estimate)
    if let Some(mut session) = current_session {
        session.end_time = session.start_time + chrono::Duration::minutes(1);
        sessions.push(session);
    }

    tracing::info!(
        events_count = events_count,
        sessions_count = sessions.len(),
        "Aggregated app events into sessions"
    );

    sessions
}

/// Execute batch insert for app usage records
async fn execute_app_usage_batch_insert(
    db: &Database,
    records: &[(String, Option<String>, DateTime<Utc>, DateTime<Utc>, Uuid)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "data.activity_app_usage",
        &[
            "app_name",
            "app_bundle_id",
            "start_time",
            "end_time",
            "source_stream_id",
            "source_table",
            "source_provider",
        ],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (app_name, app_bundle_id, start_time, end_time, stream_id) in records {
        query = query
            .bind(app_name)
            .bind(app_bundle_id)
            .bind(start_time)
            .bind(end_time)
            .bind(stream_id)
            .bind("stream_mac_apps")
            .bind("mac");
    }

    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

// ============================================================================
// MacBrowserTransform: stream_mac_browser → activity_web_browsing
// ============================================================================

/// Transform macOS browser history to activity_web_browsing ontology
pub struct MacBrowserTransform;

#[async_trait]
impl OntologyTransform for MacBrowserTransform {
    fn source_table(&self) -> &str {
        "stream_mac_browser"
    }

    fn target_table(&self) -> &str {
        "activity_web_browsing"
    }

    fn domain(&self) -> &str {
        "activity"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(
        &self,
        db: &Database,
        context: &TransformContext,
        source_id: Uuid,
    ) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting macOS Browser to activity_web_browsing transformation"
        );

        // Read stream data from data source using checkpoint
        let checkpoint_key = "mac_browser_to_activity_web_browsing";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| {
            crate::Error::Other("No data source available for transform".to_string())
        })?;
        let batches = data_source
            .read_with_checkpoint(source_id, "browser", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched macOS browser history batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(
            String,           // url
            String,           // domain
            Option<String>,   // page_title
            Option<i32>,      // visit_duration_seconds
            DateTime<Utc>,    // timestamp
            Uuid,             // source_stream_id
            serde_json::Value, // metadata
        )> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;
        let processing_start = std::time::Instant::now();

        for batch in batches {
            tracing::debug!(batch_record_count = batch.records.len(), "Processing batch");

            for record in &batch.records {
                records_read += 1;

                // Extract required fields
                let Some(url) = record.get("url").and_then(|v| v.as_str()).map(String::from) else {
                    continue;
                };

                let domain = record
                    .get("domain")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .or_else(|| extract_domain_from_url(&url));

                let Some(domain) = domain else {
                    continue;
                };

                let timestamp = record
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let page_title = record
                    .get("page_title")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let visit_duration_seconds = record
                    .get("visit_duration_seconds")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);

                // Build metadata with browser-specific fields
                let browser = record.get("browser").and_then(|v| v.as_str());
                let referrer = record.get("referrer").and_then(|v| v.as_str());
                let search_query = record.get("search_query").and_then(|v| v.as_str());
                let tab_count = record.get("tab_count").and_then(|v| v.as_i64());

                let metadata = serde_json::json!({
                    "browser": browser,
                    "referrer": referrer,
                    "search_query": search_query,
                    "tab_count": tab_count,
                });

                // Add to pending batch
                pending_records.push((
                    url,
                    domain,
                    page_title,
                    visit_duration_seconds,
                    timestamp,
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_browser_batch_insert(db, &pending_records).await;
                    let insert_duration = insert_start.elapsed();
                    batch_insert_total_ms += insert_duration.as_millis();
                    batch_insert_count += 1;

                    tracing::info!(
                        batch_size = pending_records.len(),
                        insert_duration_ms = insert_duration.as_millis(),
                        "Executed batch insert"
                    );

                    match batch_result {
                        Ok(written) => {
                            records_written += written;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                batch_size = pending_records.len(),
                                "Batch insert failed"
                            );
                            records_failed += pending_records.len();
                        }
                    }
                    pending_records.clear();
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                data_source
                    .update_checkpoint(source_id, "browser", checkpoint_key, max_ts)
                    .await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_browser_batch_insert(db, &pending_records).await;
            let insert_duration = insert_start.elapsed();
            batch_insert_total_ms += insert_duration.as_millis();
            batch_insert_count += 1;

            tracing::info!(
                batch_size = pending_records.len(),
                insert_duration_ms = insert_duration.as_millis(),
                "Executed final batch insert"
            );

            match batch_result {
                Ok(written) => {
                    records_written += written;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                    records_failed += pending_records.len();
                }
            }
        }

        let processing_duration = processing_start.elapsed();
        let total_duration = transform_start.elapsed();

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            processing_duration_ms = processing_duration.as_millis(),
            read_duration_ms = read_duration.as_millis(),
            batch_insert_total_ms,
            batch_insert_count,
            avg_batch_insert_ms = if batch_insert_count > 0 { batch_insert_total_ms / batch_insert_count as u128 } else { 0 },
            "macOS Browser to activity_web_browsing transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![],
        })
    }
}

/// Extract domain from URL string
fn extract_domain_from_url(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(String::from)
}

/// Execute batch insert for browser history records
async fn execute_browser_batch_insert(
    db: &Database,
    records: &[(String, String, Option<String>, Option<i32>, DateTime<Utc>, Uuid, serde_json::Value)],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "data.activity_web_browsing",
        &[
            "url",
            "domain",
            "page_title",
            "visit_duration_seconds",
            "timestamp",
            "source_stream_id",
            "source_table",
            "source_provider",
            "metadata",
        ],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (url, domain, page_title, visit_duration, timestamp, stream_id, metadata) in records {
        query = query
            .bind(url)
            .bind(domain)
            .bind(page_title)
            .bind(visit_duration)
            .bind(timestamp)
            .bind(stream_id)
            .bind("stream_mac_browser")
            .bind("mac")
            .bind(metadata);
    }

    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

// ============================================================================
// MacIMessageTransform: stream_mac_imessage → social_message
// ============================================================================

/// Transform macOS iMessage/SMS data to social_message ontology
pub struct MacIMessageTransform;

#[async_trait]
impl OntologyTransform for MacIMessageTransform {
    fn source_table(&self) -> &str {
        "stream_mac_imessage"
    }

    fn target_table(&self) -> &str {
        "social_message"
    }

    fn domain(&self) -> &str {
        "social"
    }

    #[tracing::instrument(skip(self, db, context), fields(source_table = %self.source_table(), target_table = %self.target_table()))]
    async fn transform(
        &self,
        db: &Database,
        context: &TransformContext,
        source_id: Uuid,
    ) -> Result<TransformResult> {
        let mut records_read = 0;
        let mut records_written = 0;
        let mut records_failed = 0;
        let mut last_processed_id: Option<Uuid> = None;

        let transform_start = std::time::Instant::now();

        tracing::info!(
            source_id = %source_id,
            "Starting macOS iMessage to social_message transformation"
        );

        // Read stream data from data source using checkpoint
        let checkpoint_key = "mac_imessage_to_social_message";
        let read_start = std::time::Instant::now();
        let data_source = context.get_data_source().ok_or_else(|| {
            crate::Error::Other("No data source available for transform".to_string())
        })?;
        let batches = data_source
            .read_with_checkpoint(source_id, "imessage", checkpoint_key)
            .await?;
        let read_duration = read_start.elapsed();

        tracing::info!(
            batch_count = batches.len(),
            read_duration_ms = read_duration.as_millis(),
            source_type = ?data_source.source_type(),
            "Fetched macOS iMessage batches from data source"
        );

        // Batch insert configuration
        let mut pending_records: Vec<(
            String,                // message_id
            Option<String>,        // thread_id
            String,                // channel
            Option<String>,        // body
            DateTime<Utc>,         // timestamp
            Option<String>,        // from_identifier
            Vec<String>,           // to_identifiers
            String,                // direction
            bool,                  // is_read
            bool,                  // is_group_message
            Option<String>,        // group_name
            bool,                  // has_attachments
            i32,                   // attachment_count
            Uuid,                  // source_stream_id
            serde_json::Value,     // metadata
        )> = Vec::new();
        let mut batch_insert_total_ms = 0u128;
        let mut batch_insert_count = 0;
        let processing_start = std::time::Instant::now();

        for batch in batches {
            tracing::debug!(batch_record_count = batch.records.len(), "Processing batch");

            for record in &batch.records {
                records_read += 1;

                // Extract required fields
                let Some(message_id) = record.get("message_id").and_then(|v| v.as_str()).map(String::from) else {
                    continue;
                };

                let timestamp = record
                    .get("date")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                    .unwrap_or_else(|| Utc::now());

                let stream_id = record
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(|| Uuid::new_v4());

                let thread_id = record
                    .get("chat_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let service = record
                    .get("service")
                    .and_then(|v| v.as_str())
                    .unwrap_or("iMessage");
                let channel = service.to_string();

                let body = record
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let is_from_me = record
                    .get("is_from_me")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let handle_id = record
                    .get("handle_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                // Determine direction and identifiers
                let direction = if is_from_me { "sent" } else { "received" };
                let from_identifier = if is_from_me { None } else { handle_id.clone() };
                let to_identifiers = if is_from_me {
                    handle_id.map(|h| vec![h]).unwrap_or_default()
                } else {
                    vec![]
                };

                let is_read = record
                    .get("is_read")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let group_name = record
                    .get("group_title")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let is_group_message = group_name.is_some();

                let has_attachments = record
                    .get("cache_has_attachments")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let attachment_count = record
                    .get("attachment_count")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;

                // Build metadata with iMessage-specific fields
                let date_read = record.get("date_read").and_then(|v| v.as_str());
                let date_delivered = record.get("date_delivered").and_then(|v| v.as_str());
                let is_delivered = record.get("is_delivered").and_then(|v| v.as_bool());
                let is_sent = record.get("is_sent").and_then(|v| v.as_bool());
                let associated_message_guid = record.get("associated_message_guid").and_then(|v| v.as_str());
                let expressive_send_style_id = record.get("expressive_send_style_id").and_then(|v| v.as_str());

                let metadata = serde_json::json!({
                    "service": service,
                    "date_read": date_read,
                    "date_delivered": date_delivered,
                    "is_delivered": is_delivered,
                    "is_sent": is_sent,
                    "associated_message_guid": associated_message_guid,
                    "expressive_send_style_id": expressive_send_style_id,
                });

                // Add to pending batch
                pending_records.push((
                    message_id,
                    thread_id,
                    channel,
                    body,
                    timestamp,
                    from_identifier,
                    to_identifiers,
                    direction.to_string(),
                    is_read,
                    is_group_message,
                    group_name,
                    has_attachments,
                    attachment_count,
                    stream_id,
                    metadata,
                ));

                last_processed_id = Some(stream_id);

                // Execute batch insert when we reach batch size
                if pending_records.len() >= BATCH_SIZE {
                    let insert_start = std::time::Instant::now();
                    let batch_result = execute_imessage_batch_insert(db, &pending_records).await;
                    let insert_duration = insert_start.elapsed();
                    batch_insert_total_ms += insert_duration.as_millis();
                    batch_insert_count += 1;

                    tracing::info!(
                        batch_size = pending_records.len(),
                        insert_duration_ms = insert_duration.as_millis(),
                        "Executed batch insert"
                    );

                    match batch_result {
                        Ok(written) => {
                            records_written += written;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                batch_size = pending_records.len(),
                                "Batch insert failed"
                            );
                            records_failed += pending_records.len();
                        }
                    }
                    pending_records.clear();
                }
            }

            // Update checkpoint after processing batch
            if let Some(max_ts) = batch.max_timestamp {
                data_source
                    .update_checkpoint(source_id, "messages", checkpoint_key, max_ts)
                    .await?;
            }
        }

        // Insert any remaining records
        if !pending_records.is_empty() {
            let insert_start = std::time::Instant::now();
            let batch_result = execute_imessage_batch_insert(db, &pending_records).await;
            let insert_duration = insert_start.elapsed();
            batch_insert_total_ms += insert_duration.as_millis();
            batch_insert_count += 1;

            tracing::info!(
                batch_size = pending_records.len(),
                insert_duration_ms = insert_duration.as_millis(),
                "Executed final batch insert"
            );

            match batch_result {
                Ok(written) => {
                    records_written += written;
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        batch_size = pending_records.len(),
                        "Final batch insert failed"
                    );
                    records_failed += pending_records.len();
                }
            }
        }

        let processing_duration = processing_start.elapsed();
        let total_duration = transform_start.elapsed();

        tracing::info!(
            source_id = %source_id,
            records_read,
            records_written,
            records_failed,
            total_duration_ms = total_duration.as_millis(),
            processing_duration_ms = processing_duration.as_millis(),
            read_duration_ms = read_duration.as_millis(),
            batch_insert_total_ms,
            batch_insert_count,
            avg_batch_insert_ms = if batch_insert_count > 0 { batch_insert_total_ms / batch_insert_count as u128 } else { 0 },
            "macOS iMessage to social_message transformation completed"
        );

        Ok(TransformResult {
            records_read,
            records_written,
            records_failed,
            last_processed_id,
            chained_transforms: vec![],
        })
    }
}

/// Execute batch insert for iMessage records
async fn execute_imessage_batch_insert(
    db: &Database,
    records: &[(
        String,
        Option<String>,
        String,
        Option<String>,
        DateTime<Utc>,
        Option<String>,
        Vec<String>,
        String,
        bool,
        bool,
        Option<String>,
        bool,
        i32,
        Uuid,
        serde_json::Value,
    )],
) -> Result<usize> {
    if records.is_empty() {
        return Ok(0);
    }

    let query_str = Database::build_batch_insert_query(
        "data.social_message",
        &[
            "message_id",
            "thread_id",
            "channel",
            "body",
            "timestamp",
            "from_identifier",
            "to_identifiers",
            "direction",
            "is_read",
            "is_group_message",
            "group_name",
            "has_attachments",
            "attachment_count",
            "source_stream_id",
            "source_table",
            "source_provider",
            "metadata",
        ],
        "source_stream_id",
        records.len(),
    );

    let mut query = sqlx::query(&query_str);

    // Bind all parameters row by row
    for (
        message_id,
        thread_id,
        channel,
        body,
        timestamp,
        from_identifier,
        to_identifiers,
        direction,
        is_read,
        is_group_message,
        group_name,
        has_attachments,
        attachment_count,
        stream_id,
        metadata,
    ) in records
    {
        query = query
            .bind(message_id)
            .bind(thread_id)
            .bind(channel)
            .bind(body)
            .bind(timestamp)
            .bind(from_identifier)
            .bind(to_identifiers)
            .bind(direction)
            .bind(is_read)
            .bind(is_group_message)
            .bind(group_name)
            .bind(has_attachments)
            .bind(attachment_count)
            .bind(stream_id)
            .bind("stream_mac_imessage")
            .bind("mac")
            .bind(metadata);
    }

    let result = query.execute(db.pool()).await?;
    Ok(result.rows_affected() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_apps_transform_metadata() {
        let transform = MacAppsTransform;
        assert_eq!(transform.source_table(), "stream_mac_apps");
        assert_eq!(transform.target_table(), "activity_app_usage");
        assert_eq!(transform.domain(), "activity");
    }

    #[test]
    fn test_mac_browser_transform_metadata() {
        let transform = MacBrowserTransform;
        assert_eq!(transform.source_table(), "stream_mac_browser");
        assert_eq!(transform.target_table(), "activity_web_browsing");
        assert_eq!(transform.domain(), "activity");
    }

    #[test]
    fn test_mac_imessage_transform_metadata() {
        let transform = MacIMessageTransform;
        assert_eq!(transform.source_table(), "stream_mac_imessage");
        assert_eq!(transform.target_table(), "social_message");
        assert_eq!(transform.domain(), "social");
    }

    #[test]
    fn test_domain_extraction() {
        assert_eq!(
            extract_domain_from_url("https://github.com/ariata-os/ariata"),
            Some("github.com".to_string())
        );
        assert_eq!(
            extract_domain_from_url("http://example.com/path?query=1"),
            Some("example.com".to_string())
        );
        assert_eq!(extract_domain_from_url("not a url"), None);
    }

    #[test]
    fn test_event_aggregation() {
        let events = vec![
            AppEvent {
                timestamp: "2024-01-01T10:00:00Z".parse().unwrap(),
                event_type: "focus_gained".to_string(),
                app_name: "VSCode".to_string(),
                bundle_id: Some("com.microsoft.VSCode".to_string()),
                stream_id: Uuid::new_v4(),
            },
            AppEvent {
                timestamp: "2024-01-01T10:05:00Z".parse().unwrap(),
                event_type: "focus_lost".to_string(),
                app_name: "VSCode".to_string(),
                bundle_id: Some("com.microsoft.VSCode".to_string()),
                stream_id: Uuid::new_v4(),
            },
        ];

        let sessions = aggregate_app_events_to_sessions(events);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].app_name, "VSCode");
        assert_eq!(
            sessions[0].end_time.signed_duration_since(sessions[0].start_time).num_minutes(),
            5
        );
    }
}
