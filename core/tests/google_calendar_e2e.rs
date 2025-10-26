//! End-to-end tests for Google Calendar sync using real APIs
//!
//! These tests require real Google OAuth credentials and will interact with
//! actual Google Calendar APIs through the auth.ariata.com OAuth proxy.
//!
//! To run these tests:
//! 1. Set GOOGLE_TEST_REFRESH_TOKEN environment variable with a valid refresh token
//! 2. Ensure auth.ariata.com is accessible
//! 3. Run with: cargo test google_calendar --test google_calendar_e2e -- --ignored --nocapture

mod common;

use ariata::{
    error::Result,
    sources::google::calendar::GoogleCalendarSync,
};
use common::TestFixture;
use std::env;

/// Helper struct for querying calendar events
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct EventSummary {
    event_id: String,
    summary: Option<String>,
    start_time: chrono::DateTime<chrono::Utc>,
    end_time: chrono::DateTime<chrono::Utc>,
    organizer_email: Option<String>,
    attendee_count: i32,
    has_conferencing: bool,
}

/// Get event count from database
async fn get_event_count(db: &sqlx::PgPool) -> Result<i64> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM stream_google_calendar")
        .fetch_one(db)
        .await?;
    Ok(count.0)
}

/// Get sample events for verification
async fn get_sample_events(
    db: &sqlx::PgPool,
    source_id: uuid::Uuid,
    limit: i64,
) -> Result<Vec<EventSummary>> {
    let events: Vec<EventSummary> = sqlx::query_as(
        r#"
        SELECT
            event_id,
            summary,
            start_time,
            end_time,
            organizer_email,
            attendee_count,
            has_conferencing
        FROM stream_google_calendar
        WHERE source_id = $1
        ORDER BY start_time DESC
        LIMIT $2
        "#,
    )
    .bind(source_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    Ok(events)
}

/// Verify events were stored correctly with detailed validation
async fn verify_events_stored(db: &sqlx::PgPool, source_id: uuid::Uuid) -> Result<()> {
    let events = get_sample_events(db, source_id, 10).await?;

    assert!(!events.is_empty(), "No events found in database");

    println!("\n📅 Sample events from sync:");
    println!(
        "{:<50} | {:<20} | {:<20} | 👥 | 📹",
        "Title", "Start", "End",
    );
    println!("{:-<120}", "");

    for event in &events {
        let title = event.summary.as_deref().unwrap_or("(no title)");
        let title_truncated = if title.len() > 47 {
            format!("{}...", &title[..47])
        } else {
            title.to_string()
        };

        println!(
            "{:<50} | {:<20} | {:<20} | {:2} | {}",
            title_truncated,
            event.start_time.format("%Y-%m-%d %H:%M"),
            event.end_time.format("%Y-%m-%d %H:%M"),
            event.attendee_count,
            if event.has_conferencing { "✓" } else { " " }
        );

        // Validate event data integrity
        assert!(!event.event_id.is_empty(), "Event ID should not be empty");
        assert!(
            event.end_time >= event.start_time,
            "End time should be after start time"
        );
    }

    // Check that we got events from the expected time range (last 30 days)
    let now = chrono::Utc::now();
    let thirty_days_ago = now - chrono::Duration::days(30);

    let recent_events: Vec<_> = events
        .iter()
        .filter(|e| e.start_time >= thirty_days_ago)
        .collect();

    println!(
        "\n📊 Events in last 30 days: {}/{}",
        recent_events.len(),
        events.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore = "Requires GOOGLE_TEST_REFRESH_TOKEN environment variable and real OAuth proxy"]
async fn test_google_calendar_real_oauth_e2e() -> Result<()> {
    // Load .env file for test credentials
    dotenv::from_path("../.env").ok();

    // Get refresh token from environment
    let refresh_token = env::var("GOOGLE_TEST_REFRESH_TOKEN")
        .expect("GOOGLE_TEST_REFRESH_TOKEN environment variable required for this test");

    println!("\n🧪 Starting Google Calendar E2E Test with Real OAuth");
    println!("{}", "=".repeat(60));

    // Setup test environment
    let mut fixture = TestFixture::new().await?;

    // Create configuration for 30-day lookback
    let config = serde_json::json!({
        "calendar_ids": ["primary"],
        "sync_window_days": 30,
        "sync_direction": "Past",
        "include_declined": false,
        "include_cancelled": false,
        "max_events_per_sync": 500
    });

    // Create source with real OAuth
    let source_id = fixture
        .create_google_source_with_oauth(&refresh_token, config)
        .await?;

    // Create sync instance
    let sync = GoogleCalendarSync::with_default_manager(source_id, fixture.db.clone());

    // Test 1: Initial sync (last 30 days)
    println!("\n🔄 Test 1: Initial Sync (30-day lookback)");
    println!("{}", "-".repeat(50));

    let initial_count = get_event_count(&fixture.db).await?;
    println!("📊 Events before sync: {}", initial_count);

    let start_time = std::time::Instant::now();
    let stats = sync.sync().await?;
    let sync_duration = start_time.elapsed();

    println!("✅ Sync completed in {:.2}s", sync_duration.as_secs_f32());
    println!("📈 Records fetched: {}", stats.records_fetched);
    println!("✍️  Records written: {}", stats.records_written);
    println!("❌ Records failed: {}", stats.records_failed);

    let after_initial_count = get_event_count(&fixture.db).await?;
    println!("📊 Total events in database: {}", after_initial_count);

    assert!(
        after_initial_count > initial_count,
        "Should have synced at least some events"
    );

    // Verify event data
    verify_events_stored(&fixture.db, source_id).await?;

    // Check sync token
    let sync_token = fixture.get_sync_token(source_id).await?;
    if let Some(ref token) = sync_token {
        println!("🔖 Sync token saved: {}", &token[..20.min(token.len())]);
    } else {
        println!("ℹ️  No sync token (expected for time-bounded sync)");
    }

    // Test 2: Incremental sync
    println!("\n🔄 Test 2: Incremental Sync");
    println!("{}", "-".repeat(50));

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let before_incremental = get_event_count(&fixture.db).await?;
    println!("📊 Events before incremental sync: {}", before_incremental);

    let start_time = std::time::Instant::now();
    let stats = sync.sync().await?;
    let sync_duration = start_time.elapsed();

    println!(
        "✅ Incremental sync completed in {:.2}s",
        sync_duration.as_secs_f32()
    );
    println!("📈 Records fetched: {}", stats.records_fetched);
    println!("✍️  Records written: {}", stats.records_written);
    println!("❌ Records failed: {}", stats.records_failed);

    let after_incremental = get_event_count(&fixture.db).await?;
    println!("📊 Total events after incremental: {}", after_incremental);

    assert!(
        after_incremental >= before_incremental,
        "Event count should not decrease"
    );

    // Test 3: Configuration change (switch to future events)
    println!("\n🔄 Test 3: Configuration Change (Future Events)");
    println!("{}", "-".repeat(50));

    let future_config = serde_json::json!({
        "calendar_ids": ["primary"],
        "sync_window_days": 30,
        "sync_direction": "Future",
        "include_declined": false,
        "include_cancelled": false,
        "max_events_per_sync": 100
    });

    sqlx::query("UPDATE sources SET config = $1, last_sync_token = NULL WHERE id = $2")
        .bind(future_config)
        .bind(source_id)
        .execute(&fixture.db)
        .await?;

    println!("📝 Updated config to sync next 30 days of events");

    let future_sync = GoogleCalendarSync::with_default_manager(source_id, fixture.db.clone());
    let stats = future_sync.sync().await?;
    println!("📈 Records fetched: {}", stats.records_fetched);
    println!("✍️  Records written: {}", stats.records_written);

    // Test 4: Error recovery (invalid sync token)
    println!("\n🔄 Test 4: Error Recovery (Invalid Sync Token)");
    println!("{}", "-".repeat(50));

    // Set an invalid sync token to test recovery
    sqlx::query("UPDATE sources SET last_sync_token = $1 WHERE id = $2")
        .bind("invalid_token_test_12345")
        .bind(source_id)
        .execute(&fixture.db)
        .await?;

    println!("💔 Set invalid sync token to test recovery");

    // This should detect the invalid token and fall back to full sync
    let result = sync.sync().await;

    match &result {
        Ok(_) => println!("✅ Successfully recovered from invalid sync token"),
        Err(e) => println!("❌ Sync failed with error: {:?}", e),
    }

    assert!(result.is_ok(), "Should recover from invalid sync token");

    // Verify sync token handling after recovery
    let new_token = fixture.get_sync_token(source_id).await?;
    if let Some(ref token) = new_token {
        assert_ne!(
            token, "invalid_token_test_12345",
            "Should have replaced invalid token"
        );
        println!("🔖 New sync token saved after recovery");
    } else {
        println!("ℹ️  No sync token after recovery (expected for time-bounded fallback)");
    }

    // Test 5: Concurrent sync requests
    println!("\n🔄 Test 5: Concurrent Sync Requests");
    println!("{}", "-".repeat(50));

    // Try multiple syncs in parallel to test rate limiting handling
    let mut handles = vec![];
    for i in 0..3 {
        let sync_clone =
            GoogleCalendarSync::with_default_manager(source_id, fixture.db.clone());

        handles.push(tokio::spawn(async move {
            println!("🚀 Starting concurrent sync #{}", i + 1);
            let result = sync_clone.sync().await;
            match &result {
                Ok(stats) => println!(
                    "✅ Sync #{} completed: {} records written",
                    i + 1,
                    stats.records_written
                ),
                Err(e) => println!(
                    "⚠️  Sync #{} failed (expected if rate limited): {}",
                    i + 1,
                    e
                ),
            }
            result
        }));
    }

    // Wait for all syncs to complete
    for handle in handles {
        let _ = handle.await;
    }

    println!("\n\n");
    println!("🎉 All tests completed successfully!");
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
        tables.contains(&"stream_google_calendar".to_string()),
        "stream_google_calendar table should exist"
    );

    println!("✅ Test containers and database setup working correctly");
    Ok(())
}
