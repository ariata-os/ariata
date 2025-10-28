//! Integration tests for the public API
//!
//! Tests the sync_stream() API and other library functions using TestFixture

mod common;

use ariata::{
    error::Result,
    sources::{SourceAuth, StreamFactory},
    sync_stream,
};
use common::TestFixture;
use uuid::Uuid;
use std::sync::Arc;

/// Test syncing a stream using the public API
#[tokio::test]
async fn test_sync_stream_api() -> Result<()> {
    let fixture = TestFixture::new().await?;

    // Create a mock Google source
    let source_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO sources (id, type, name, is_active, created_at, updated_at)
        VALUES ($1, 'google', 'Test Google Source', true, NOW(), NOW())
        "#
    )
    .bind(source_id)
    .execute(&fixture.db)
    .await?;

    // Create a calendar stream entry
    sqlx::query(
        r#"
        INSERT INTO streams (source_id, stream_name, table_name, is_enabled, config)
        VALUES ($1, 'calendar', 'stream_google_calendar', true, '{"calendar_ids": ["primary"]}')
        "#
    )
    .bind(source_id)
    .execute(&fixture.db)
    .await?;

    // Note: This will fail without real OAuth tokens, but it tests the API contract
    let result = sync_stream(&fixture.db, source_id, "calendar", None).await;

    // We expect an auth error since we don't have real tokens
    assert!(result.is_err());
    match result {
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(
                error_str.contains("token") || error_str.contains("auth"),
                "Should fail with auth-related error, got: {}",
                error_str
            );
        }
        Ok(_) => panic!("Should have failed without valid OAuth tokens"),
    }

    Ok(())
}

/// Test StreamFactory instantiation
#[tokio::test]
async fn test_stream_factory() -> Result<()> {
    let fixture = TestFixture::new().await?;

    // Create a test source
    let source_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO sources (id, type, name, is_active, created_at, updated_at)
        VALUES ($1, 'google', 'Test Factory Source', true, NOW(), NOW())
        "#
    )
    .bind(source_id)
    .execute(&fixture.db)
    .await?;

    let factory = StreamFactory::new(fixture.db.clone());

    // Test creating a Google Calendar stream
    let stream = factory.create_stream(source_id, "calendar").await?;
    assert_eq!(stream.stream_name(), "calendar");
    assert_eq!(stream.source_name(), "google");
    assert_eq!(stream.table_name(), "stream_google_calendar");

    // Test creating a Gmail stream
    let stream = factory.create_stream(source_id, "gmail").await?;
    assert_eq!(stream.stream_name(), "gmail");
    assert_eq!(stream.source_name(), "google");
    assert_eq!(stream.table_name(), "stream_google_gmail");

    // Test unknown stream
    let result = factory.create_stream(source_id, "nonexistent").await;
    assert!(result.is_err());

    Ok(())
}

/// Test listing sources
#[tokio::test]
async fn test_list_sources() -> Result<()> {
    let fixture = TestFixture::new().await?;

    // Initially empty
    let sources = ariata::list_sources(&fixture.db).await?;
    assert_eq!(sources.len(), 0);

    // Create some test sources
    for i in 0..3 {
        sqlx::query(
            r#"
            INSERT INTO sources (id, type, name, is_active, created_at, updated_at)
            VALUES ($1, 'google', $2, true, NOW(), NOW())
            "#
        )
        .bind(Uuid::new_v4())
        .bind(format!("Test Source {}", i))
        .execute(&fixture.db)
        .await?;
    }

    let sources = ariata::list_sources(&fixture.db).await?;
    assert_eq!(sources.len(), 3);

    // Verify source properties
    for source in &sources {
        assert_eq!(source.source_type, "google");
        assert!(source.is_active);
        assert!(source.name.starts_with("Test Source"));
    }

    Ok(())
}

/// Test source status and sync history
#[tokio::test]
async fn test_source_status() -> Result<()> {
    let fixture = TestFixture::new().await?;

    let source_id = Uuid::new_v4();

    // Create source
    sqlx::query(
        r#"
        INSERT INTO sources (id, type, name, is_active, created_at, updated_at)
        VALUES ($1, 'google', 'Status Test Source', true, NOW(), NOW())
        "#
    )
    .bind(source_id)
    .execute(&fixture.db)
    .await?;

    // Get status (should have zero syncs)
    let status = ariata::get_source_status(&fixture.db, source_id).await?;
    assert_eq!(status.total_syncs, 0);
    assert_eq!(status.successful_syncs, 0);
    assert_eq!(status.failed_syncs, 0);

    // Add some sync logs
    for i in 0..5 {
        sqlx::query(
            r#"
            INSERT INTO sync_logs (
                id, source_id, sync_mode, started_at, completed_at,
                duration_ms, status, records_fetched, records_written,
                created_at
            )
            VALUES ($1, $2, 'full_refresh', NOW(), NOW(), $3, $4, 100, 100, NOW())
            "#
        )
        .bind(Uuid::new_v4())
        .bind(source_id)
        .bind(100 + i * 50)
        .bind(if i < 4 { "success" } else { "failed" })
        .execute(&fixture.db)
        .await?;
    }

    // Get updated status
    let status = ariata::get_source_status(&fixture.db, source_id).await?;
    assert_eq!(status.total_syncs, 5);
    assert_eq!(status.successful_syncs, 4);
    assert_eq!(status.failed_syncs, 1);

    // Test sync history
    let history = ariata::get_sync_history(&fixture.db, source_id, 10).await?;
    assert_eq!(history.len(), 5);

    // Verify history is sorted by started_at DESC
    for i in 1..history.len() {
        assert!(history[i - 1].started_at >= history[i].started_at);
    }

    Ok(())
}

/// Test database schema (tables exist)
#[tokio::test]
async fn test_database_schema() -> Result<()> {
    let fixture = TestFixture::new().await?;

    let tables = fixture.get_tables().await?;

    // Core tables
    assert!(tables.contains(&"sources".to_string()));
    assert!(tables.contains(&"streams".to_string()));
    assert!(tables.contains(&"sync_logs".to_string()));

    // Stream tables
    assert!(tables.contains(&"stream_google_calendar".to_string()));
    assert!(tables.contains(&"stream_google_gmail".to_string()));
    // Note: stream_notion_pages doesn't have a migration yet

    Ok(())
}
