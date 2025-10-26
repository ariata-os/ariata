//! Example of syncing Gmail messages
//!
//! This demonstrates how to use the GoogleGmailSync to fetch and store
//! email messages from Gmail.

use anyhow::Result;
use ariata::{
    oauth::token_manager::TokenManager,
    sources::google::{GoogleGmailSync, GoogleGmailConfig},
};
use sqlx::PgPool;
use std::sync::Arc;
use tracing_subscriber;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Connect to PostgreSQL
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/ariata".to_string());

    let pool = PgPool::connect(&database_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Create a source ID (in production, this would be fetched from the sources table)
    let source_id = Uuid::new_v4();

    // Insert a test source with OAuth tokens
    // In production, these would be obtained through the OAuth flow
    sqlx::query(
        r#"
        INSERT INTO sources (id, name, type, config, access_token, refresh_token)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (id) DO UPDATE SET
            access_token = EXCLUDED.access_token,
            refresh_token = EXCLUDED.refresh_token
        "#
    )
    .bind(source_id)
    .bind("My Gmail")
    .bind("google_gmail")
    .bind(serde_json::json!({
        "label_ids": ["INBOX", "SENT"],
        "sync_window_days": 30,
        "max_messages_per_sync": 100,
        "fetch_body": true
    }))
    .bind("your_access_token_here")  // Replace with actual token
    .bind("your_refresh_token_here") // Replace with actual token
    .execute(&pool)
    .await?;

    // Create token manager and Gmail sync instance
    let token_manager = Arc::new(TokenManager::new(pool.clone()));

    // Create custom configuration
    let config = GoogleGmailConfig {
        label_ids: vec!["INBOX".to_string(), "SENT".to_string()],
        sync_window_days: 30,
        max_messages_per_sync: 100,
        fetch_body: true,
        include_spam_trash: false,
        query: Some("is:important".to_string()), // Only sync important emails
        ..Default::default()
    };

    let gmail_sync = GoogleGmailSync::with_config(
        source_id,
        pool.clone(),
        token_manager,
        config
    );

    // Perform sync
    println!("Starting Gmail sync...");
    match gmail_sync.sync().await {
        Ok(result) => {
            println!("Sync completed successfully!");
            println!("  Messages fetched: {}", result.records_fetched);
            println!("  Messages written: {}", result.records_written);
            println!("  Messages failed: {}", result.records_failed);
            println!("  Duration: {}ms", result.duration_ms());

            if let Some(cursor) = result.next_cursor {
                println!("  Next sync cursor: {}", cursor);
            }
        }
        Err(e) => {
            eprintln!("Sync failed: {}", e);
        }
    }

    // Query some synced messages
    let messages = sqlx::query!(
        r#"
        SELECT
            message_id,
            subject,
            from_email,
            date,
            snippet,
            is_unread,
            is_important,
            array_length(attachment_names, 1) as attachment_count
        FROM stream_google_gmail
        WHERE source_id = $1
        ORDER BY date DESC
        LIMIT 10
        "#,
        source_id
    )
    .fetch_all(&pool)
    .await?;

    println!("\nRecent messages:");
    for msg in messages {
        println!("---");
        println!("ID: {}", msg.message_id);
        println!("Subject: {}", msg.subject.unwrap_or_else(|| "(No subject)".to_string()));
        println!("From: {}", msg.from_email.unwrap_or_else(|| "(Unknown)".to_string()));
        println!("Date: {}", msg.date);
        println!("Snippet: {}", msg.snippet.unwrap_or_else(|| "(Empty)".to_string()));

        if msg.is_unread.unwrap_or(false) {
            println!("Status: UNREAD");
        }

        if msg.is_important.unwrap_or(false) {
            println!("Status: IMPORTANT");
        }

        if let Some(count) = msg.attachment_count {
            if count > 0 {
                println!("Attachments: {}", count);
            }
        }
    }

    // Show thread statistics
    let thread_stats = sqlx::query!(
        r#"
        SELECT
            thread_id,
            COUNT(*) as message_count,
            MIN(date) as first_message,
            MAX(date) as last_message
        FROM stream_google_gmail
        WHERE source_id = $1
        GROUP BY thread_id
        HAVING COUNT(*) > 1
        ORDER BY COUNT(*) DESC
        LIMIT 5
        "#,
        source_id
    )
    .fetch_all(&pool)
    .await?;

    if !thread_stats.is_empty() {
        println!("\nTop conversation threads:");
        for thread in thread_stats {
            println!("  Thread {}: {} messages ({} to {})",
                thread.thread_id,
                thread.message_count.unwrap_or(0),
                thread.first_message.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "?".to_string()),
                thread.last_message.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "?".to_string())
            );
        }
    }

    Ok(())
}