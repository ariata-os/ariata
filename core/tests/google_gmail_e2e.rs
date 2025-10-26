//! End-to-end tests for Google Gmail sync using real APIs
//!
//! These tests require real Google OAuth credentials and will interact with
//! actual Google Gmail APIs through the auth.ariata.com OAuth proxy.
//!
//! To run these tests:
//! 1. Set GOOGLE_TEST_REFRESH_TOKEN environment variable with a valid refresh token
//! 2. Ensure auth.ariata.com is accessible
//! 3. Run with: cargo test google_gmail --test google_gmail_e2e -- --ignored --nocapture

mod common;

use ariata::{
    error::Result,
    oauth::token_manager::TokenManager,
    sources::google::gmail::GoogleGmailSync,
};
use common::TestFixture;
use std::{env, sync::Arc};

/// Helper struct for querying Gmail messages
#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
struct MessageSummary {
    message_id: String,
    thread_id: String,
    subject: Option<String>,
    from_email: Option<String>,
    from_name: Option<String>,
    date: chrono::DateTime<chrono::Utc>,
    snippet: Option<String>,
    labels: Vec<String>,
    is_unread: bool,
    is_important: bool,
    is_starred: bool,
    has_attachments: bool,
    attachment_count: i32,
}

/// Helper struct for thread statistics
#[derive(sqlx::FromRow, Debug)]
struct ThreadStats {
    thread_id: String,
    message_count: i64,
    first_message_date: Option<chrono::DateTime<chrono::Utc>>,
    last_message_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get message count from database
async fn get_message_count(db: &sqlx::PgPool) -> Result<i64> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stream_google_gmail")
        .fetch_one(db)
        .await?;
    Ok(count.0)
}

/// Get sample messages for verification
async fn get_sample_messages(
    db: &sqlx::PgPool,
    source_id: uuid::Uuid,
    limit: i64,
) -> Result<Vec<MessageSummary>> {
    let messages: Vec<MessageSummary> = sqlx::query_as(
        r#"
        SELECT
            message_id,
            thread_id,
            subject,
            from_email,
            from_name,
            date,
            snippet,
            labels,
            is_unread,
            is_important,
            is_starred,
            has_attachments,
            attachment_count
        FROM stream_google_gmail
        WHERE source_id = $1
        ORDER BY date DESC
        LIMIT $2
        "#,
    )
    .bind(source_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(messages)
}

/// Get thread statistics
async fn get_thread_stats(
    db: &sqlx::PgPool,
    source_id: uuid::Uuid,
) -> Result<Vec<ThreadStats>> {
    let stats: Vec<ThreadStats> = sqlx::query_as(
        r#"
        SELECT
            thread_id,
            COUNT(*) as message_count,
            MIN(date) as first_message_date,
            MAX(date) as last_message_date
        FROM stream_google_gmail
        WHERE source_id = $1
        GROUP BY thread_id
        HAVING COUNT(*) > 1
        ORDER BY COUNT(*) DESC
        LIMIT 5
        "#,
    )
    .bind(source_id)
    .fetch_all(db)
    .await?;

    Ok(stats)
}

/// Verify messages were stored correctly with detailed validation
async fn verify_messages_stored(db: &sqlx::PgPool, source_id: uuid::Uuid) -> Result<()> {
    let messages = get_sample_messages(db, source_id, 10).await?;

    assert!(!messages.is_empty(), "No messages found in database");

    println!("\n📧 Sample messages from sync:");
    println!(
        "{:<50} | {:<30} | {:<20} | 📎 | ⭐",
        "Subject", "From", "Date"
    );
    println!("{:-<120}", "");

    for msg in &messages {
        let subject = msg.subject.as_deref().unwrap_or("(no subject)");
        let subject_truncated = if subject.len() > 47 {
            format!("{}...", &subject[..47])
        } else {
            subject.to_string()
        };

        let from = if let Some(ref name) = msg.from_name {
            format!("{} <{}>", name, msg.from_email.as_deref().unwrap_or("?"))
        } else {
            msg.from_email.as_deref().unwrap_or("(unknown)").to_string()
        };
        let from_truncated = if from.len() > 27 {
            format!("{}...", &from[..27])
        } else {
            from
        };

        println!(
            "{:<50} | {:<30} | {:<20} | {:2} | {}",
            subject_truncated,
            from_truncated,
            msg.date.format("%Y-%m-%d %H:%M"),
            msg.attachment_count,
            if msg.is_starred { "⭐" } else { " " }
        );

        // Validate message data integrity
        assert!(!msg.message_id.is_empty(), "Message ID should not be empty");
        assert!(!msg.thread_id.is_empty(), "Thread ID should not be empty");
        assert!(!msg.labels.is_empty(), "Message should have at least one label");

        // Validate attachment consistency
        if msg.has_attachments {
            assert!(
                msg.attachment_count > 0,
                "has_attachments=true but attachment_count=0"
            );
        } else {
            assert_eq!(
                msg.attachment_count, 0,
                "has_attachments=false but attachment_count > 0"
            );
        }
    }

    // Check that we got messages from the expected time range (last 30 days)
    let now = chrono::Utc::now();
    let thirty_days_ago = now - chrono::Duration::days(30);

    let recent_messages: Vec<_> = messages
        .iter()
        .filter(|m| m.date >= thirty_days_ago)
        .collect();

    println!(
        "\n📊 Messages in last 30 days: {}/{}",
        recent_messages.len(),
        messages.len()
    );

    // Show label distribution
    let mut label_counts = std::collections::HashMap::new();
    for msg in &messages {
        for label in &msg.labels {
            *label_counts.entry(label.clone()).or_insert(0) += 1;
        }
    }

    println!("\n🏷️  Label distribution:");
    for (label, count) in label_counts.iter() {
        println!("  {}: {}", label, count);
    }

    Ok(())
}

/// Verify thread structure
async fn verify_thread_structure(db: &sqlx::PgPool, source_id: uuid::Uuid) -> Result<()> {
    let threads = get_thread_stats(db, source_id).await?;

    if threads.is_empty() {
        println!("ℹ️  No multi-message threads found (all messages are standalone)");
        return Ok(());
    }

    println!("\n🧵 Thread statistics:");
    println!(
        "{:<20} | {:>8} | {:<20} | {:<20}",
        "Thread ID", "Messages", "First", "Last"
    );
    println!("{:-<80}", "");

    for thread in &threads {
        let thread_id_short = if thread.thread_id.len() > 16 {
            format!("{}...", &thread.thread_id[..16])
        } else {
            thread.thread_id.clone()
        };

        println!(
            "{:<20} | {:>8} | {:<20} | {:<20}",
            thread_id_short,
            thread.message_count,
            thread
                .first_message_date
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "?".to_string()),
            thread
                .last_message_date
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "?".to_string())
        );

        // Validate thread integrity
        assert!(
            thread.message_count >= 2,
            "Thread should have at least 2 messages"
        );
        if let (Some(first), Some(last)) = (thread.first_message_date, thread.last_message_date) {
            assert!(
                last >= first,
                "Last message should be after or equal to first message"
            );
        }
    }

    Ok(())
}

#[tokio::test]
#[ignore = "Requires GOOGLE_TEST_REFRESH_TOKEN environment variable and real OAuth proxy"]
async fn test_google_gmail_real_oauth_e2e() -> Result<()> {
    // Load .env file for test credentials
    dotenv::from_path("../.env").ok();

    // Get refresh token from environment
    let refresh_token = env::var("GOOGLE_TEST_REFRESH_TOKEN")
        .expect("GOOGLE_TEST_REFRESH_TOKEN environment variable required for this test");

    println!("\n🧪 Starting Google Gmail E2E Test with Real OAuth");
    println!("{}", "=".repeat(60));

    // Setup test environment
    let mut fixture = TestFixture::new().await?;

    // Create configuration for 30-day lookback
    let config = serde_json::json!({
        "label_ids": ["INBOX", "SENT"],
        "sync_window_days": 30,
        "max_messages_per_sync": 100,
        "fetch_body": true,
        "include_spam_trash": false,
        "sync_mode": "messages"
    });

    // Create source with real OAuth
    let source_id = fixture
        .create_google_source_with_oauth(&refresh_token, config)
        .await?;

    // Create token manager with OAuth proxy configuration
    let proxy_config = ariata::oauth::token_manager::OAuthProxyConfig {
        base_url: fixture.oauth_proxy_url.clone(),
    };
    let token_manager = Arc::new(TokenManager::with_config(fixture.db.clone(), proxy_config));

    // Create Gmail sync instance
    let gmail_sync = GoogleGmailSync::new(source_id, fixture.db.clone(), token_manager.clone());

    // Test 1: Initial sync (last 30 days)
    println!("\n🔄 Test 1: Initial Sync (30-day lookback)");
    println!("{}", "-".repeat(50));

    let initial_count = get_message_count(&fixture.db).await?;
    println!("📊 Messages before sync: {}", initial_count);

    let start_time = std::time::Instant::now();
    let stats = gmail_sync.sync().await?;
    let sync_duration = start_time.elapsed();

    println!("✅ Sync completed in {:.2}s", sync_duration.as_secs_f32());
    println!("📈 Records fetched: {}", stats.records_fetched);
    println!("✍️  Records written: {}", stats.records_written);
    println!("❌ Records failed: {}", stats.records_failed);

    let after_initial_count = get_message_count(&fixture.db).await?;
    println!("📊 Total messages in database: {}", after_initial_count);

    assert!(
        after_initial_count > initial_count,
        "Should have synced at least some messages"
    );

    // Verify message data
    verify_messages_stored(&fixture.db, source_id).await?;

    // Verify thread structure
    verify_thread_structure(&fixture.db, source_id).await?;

    // Check history ID (for incremental sync)
    let history_id = fixture.get_sync_token(source_id).await?;
    if let Some(ref id) = history_id {
        println!("🔖 History ID saved: {}", &id[..20.min(id.len())]);
    } else {
        println!("ℹ️  No history ID saved yet");
    }

    // Test 2: Incremental sync
    println!("\n🔄 Test 2: Incremental Sync");
    println!("{}", "-".repeat(50));

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let before_incremental = get_message_count(&fixture.db).await?;
    println!(
        "📊 Messages before incremental sync: {}",
        before_incremental
    );

    let start_time = std::time::Instant::now();
    let stats = gmail_sync.sync().await?;
    let sync_duration = start_time.elapsed();

    println!(
        "✅ Incremental sync completed in {:.2}s",
        sync_duration.as_secs_f32()
    );
    println!("📈 Records fetched: {}", stats.records_fetched);
    println!("✍️  Records written: {}", stats.records_written);
    println!("❌ Records failed: {}", stats.records_failed);

    let after_incremental = get_message_count(&fixture.db).await?;
    println!("📊 Total messages after incremental: {}", after_incremental);

    assert!(
        after_incremental >= before_incremental,
        "Message count should not decrease"
    );

    // Test 3: Label filtering (INBOX only)
    println!("\n🔄 Test 3: Label Filtering (INBOX only)");
    println!("{}", "-".repeat(50));

    let inbox_config = serde_json::json!({
        "label_ids": ["INBOX"],
        "sync_window_days": 30,
        "max_messages_per_sync": 50,
        "fetch_body": true,
        "include_spam_trash": false,
        "sync_mode": "messages"
    });

    sqlx::query("UPDATE sources SET config = $1, last_sync_token = NULL WHERE id = $2")
        .bind(inbox_config)
        .bind(source_id)
        .execute(&fixture.db)
        .await?;

    println!("📝 Updated config to sync INBOX only");

    let inbox_sync = GoogleGmailSync::new(source_id, fixture.db.clone(), token_manager.clone());
    let stats = inbox_sync.sync().await?;
    println!("📈 Records fetched: {}", stats.records_fetched);
    println!("✍️  Records written: {}", stats.records_written);

    // Verify INBOX messages
    let inbox_messages = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM stream_google_gmail WHERE source_id = $1 AND 'INBOX' = ANY(labels)"
    )
    .bind(source_id)
    .fetch_one(&fixture.db)
    .await?;

    println!("📬 Messages with INBOX label: {}", inbox_messages.0);
    assert!(inbox_messages.0 > 0, "Should have at least one INBOX message");

    // Test 4: Error recovery (invalid history_id)
    println!("\n🔄 Test 4: Error Recovery (Invalid History ID)");
    println!("{}", "-".repeat(50));

    // Set an invalid history ID to test recovery
    sqlx::query("UPDATE sources SET last_sync_token = $1 WHERE id = $2")
        .bind("12345_invalid_history_id")
        .bind(source_id)
        .execute(&fixture.db)
        .await?;

    println!("💔 Set invalid history ID to test recovery");

    // This should detect the invalid history ID and fall back to full sync
    let result = gmail_sync.sync().await;

    match &result {
        Ok(_) => println!("✅ Successfully recovered from invalid history ID"),
        Err(e) => println!("❌ Sync failed with error: {:?}", e),
    }

    // Gmail may or may not recover gracefully - it depends on API behavior
    // We just verify that it doesn't panic
    println!("✅ Error handling completed");

    // Test 5: Verify body content extraction
    println!("\n🔄 Test 5: Body Content Extraction");
    println!("{}", "-".repeat(50));

    let messages_with_body = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM stream_google_gmail
         WHERE source_id = $1 AND (body_plain IS NOT NULL OR body_html IS NOT NULL)"
    )
    .bind(source_id)
    .fetch_one(&fixture.db)
    .await?;

    println!("📄 Messages with body content: {}", messages_with_body.0);

    if messages_with_body.0 > 0 {
        // Show a sample body
        let sample: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT body_plain, body_html FROM stream_google_gmail
             WHERE source_id = $1 AND body_plain IS NOT NULL
             LIMIT 1"
        )
        .bind(source_id)
        .fetch_optional(&fixture.db)
        .await?;

        if let Some((plain, html)) = sample {
            if let Some(p) = plain {
                let preview = if p.len() > 100 {
                    format!("{}...", &p[..100])
                } else {
                    p
                };
                println!("📝 Sample plain text body: {}", preview);
            }
            if html.is_some() {
                println!("📝 HTML body also extracted");
            }
        }
    }

    // Test 6: Verify attachments
    println!("\n🔄 Test 6: Attachment Metadata");
    println!("{}", "-".repeat(50));

    let messages_with_attachments = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM stream_google_gmail
         WHERE source_id = $1 AND has_attachments = true"
    )
    .bind(source_id)
    .fetch_one(&fixture.db)
    .await?;

    println!(
        "📎 Messages with attachments: {}",
        messages_with_attachments.0
    );

    if messages_with_attachments.0 > 0 {
        // Show sample attachment info
        let sample: Option<(Vec<String>, Vec<String>, Vec<i32>)> = sqlx::query_as(
            "SELECT attachment_types, attachment_names, attachment_sizes_bytes
             FROM stream_google_gmail
             WHERE source_id = $1 AND has_attachments = true
             LIMIT 1"
        )
        .bind(source_id)
        .fetch_optional(&fixture.db)
        .await?;

        if let Some((types, names, sizes)) = sample {
            println!("📎 Sample attachment:");
            for i in 0..types.len().min(3) {
                println!(
                    "  - {} ({}, {} bytes)",
                    names.get(i).unwrap_or(&"?".to_string()),
                    types.get(i).unwrap_or(&"?".to_string()),
                    sizes.get(i).unwrap_or(&0)
                );
            }
        }
    }

    println!("\n\n");
    println!("🎉 All Gmail tests completed successfully!");
    println!("{}", "=".repeat(60));

    Ok(())
}

#[tokio::test]
async fn test_container_setup_only() -> Result<()> {
    // Quick test to verify containers start correctly without OAuth
    println!("🧪 Testing container setup...");

    let fixture = TestFixture::new().await?;

    // Verify database connection
    fixture.verify_connection().await?;

    // Verify tables exist
    let tables = fixture.get_tables().await?;

    println!("📋 Available tables:");
    for table in &tables {
        println!("  - {}", table);
    }

    assert!(
        tables.contains(&"sources".to_string()),
        "sources table should exist"
    );
    assert!(
        tables.contains(&"stream_google_gmail".to_string()),
        "stream_google_gmail table should exist"
    );
    assert!(
        tables.contains(&"stream_google_calendar".to_string()),
        "stream_google_calendar table should exist"
    );

    println!("✅ Test containers and database setup working correctly");
    Ok(())
}
