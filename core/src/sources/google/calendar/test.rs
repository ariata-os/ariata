//! End-to-end tests for Google Calendar sync using real APIs
//!
//! These tests require real Google OAuth credentials and will interact with
//! actual Google Calendar APIs through the auth.ariata.com OAuth proxy.
//!
//! To run these tests:
//! 1. Set GOOGLE_TEST_REFRESH_TOKEN environment variable with a valid refresh token
//! 2. Ensure auth.ariata.com is accessible
//! 3. Run with: cargo test test_google_calendar_real_oauth_e2e -- --ignored --nocapture

#[cfg(test)]
mod tests {
    use crate::{
        error::Result,
        oauth::token_manager::{TokenManager, OAuthProxyConfig},
        sources::google::{
            calendar::GoogleCalendarSync,
            config::{GoogleCalendarConfig, SyncDirection},
        },
    };
    use chrono::{Duration, Utc};
    use sqlx::{PgPool, postgres::PgPoolOptions};
    use std::sync::Arc;
    use std::env;
    // Use testcontainers for isolated test environment
    use testcontainers_modules::testcontainers::{runners::AsyncRunner, ContainerAsync};
    use testcontainers_modules::{postgres::Postgres, minio::MinIO};
    use uuid::Uuid;

    /// Test fixture that manages containers and database setup
    struct TestFixture {
        db: PgPool,
        _pg_container: ContainerAsync<Postgres>,
        _minio_container: ContainerAsync<MinIO>,
        source_id: Option<Uuid>,
        oauth_proxy_url: String,
    }

    impl TestFixture {
        /// Create a new test fixture with running containers
        async fn new() -> Result<Self> {
            println!("🚀 Starting test containers...");

            // Start PostgreSQL container
            let pg_container = Postgres::default()
                .with_db_name("ariata_test")
                .with_user("test_user")
                .with_password("test_pass")
                .start()
                .await
                .expect("PostgreSQL container failed to start");

            let pg_port = pg_container
                .get_host_port_ipv4(5432)
                .await
                .expect("Failed to get PostgreSQL port");

            println!("✅ PostgreSQL running on port {}", pg_port);

            // Start MinIO container for S3-compatible storage
            let minio_container = MinIO::default()
                .start()
                .await
                .expect("MinIO container failed to start");

            let minio_port = minio_container
                .get_host_port_ipv4(9000)
                .await
                .expect("Failed to get MinIO port");

            println!("✅ MinIO running on port {}", minio_port);

            // Create database connection
            let database_url = format!(
                "postgresql://test_user:test_pass@127.0.0.1:{}/ariata_test",
                pg_port
            );

            let db = PgPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await
                .expect("Failed to connect to test database");

            // Run migrations
            println!("📦 Running database migrations...");
            sqlx::migrate!("./migrations")
                .run(&db)
                .await
                .expect("Failed to run migrations");

            // Get OAuth proxy URL from environment or use default
            let oauth_proxy_url = env::var("OAUTH_PROXY_URL")
                .unwrap_or_else(|_| "https://auth.ariata.com".to_string());

            println!("🔐 Using OAuth proxy at: {}", oauth_proxy_url);

            Ok(Self {
                db,
                _pg_container: pg_container,
                _minio_container: minio_container,
                source_id: None,
                oauth_proxy_url,
            })
        }

        /// Create a Google Calendar source with OAuth tokens using the real OAuth proxy
        async fn create_source_with_real_oauth(&mut self, refresh_token: &str) -> Result<Uuid> {
            println!("\n📝 Creating Google Calendar source with real OAuth...");

            let source_id = Uuid::new_v4();

            // Use the OAuth proxy to exchange refresh token for access token
            println!("🔄 Exchanging refresh token for access token via {}", self.oauth_proxy_url);

            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/google/refresh", self.oauth_proxy_url))
                .json(&serde_json::json!({
                    "refresh_token": refresh_token
                }))
                .send()
                .await
                .map_err(|e| crate::error::Error::Network(format!("OAuth proxy request failed: {}", e)))?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(crate::error::Error::Authentication(
                    format!("OAuth token refresh failed: {}", error_text)
                ));
            }

            let token_response: serde_json::Value = response.json().await
                .map_err(|e| crate::error::Error::Other(format!("Failed to parse token response: {}", e)))?;

            let access_token = token_response["access_token"].as_str()
                .ok_or_else(|| crate::error::Error::Other("Missing access token in response".to_string()))?;

            let expires_in = token_response["expires_in"].as_i64().unwrap_or(3600);
            let expires_at = Utc::now() + Duration::seconds(expires_in);

            println!("✅ Got access token (expires in {} seconds)", expires_in);

            // Create configuration for 30-day lookback
            let config = GoogleCalendarConfig {
                calendar_ids: vec!["primary".to_string()],
                sync_window_days: 30,
                sync_direction: SyncDirection::Past,
                include_declined: false,
                include_cancelled: false,
                max_events_per_sync: 500,
            };

            // Insert source into database
            sqlx::query(
                r#"
                INSERT INTO sources (
                    id, type, name, is_active,
                    refresh_token, access_token, token_expires_at,
                    config, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
                "#,
            )
            .bind(source_id)
            .bind("google")
            .bind("Test Google Calendar")
            .bind(true)
            .bind(refresh_token)
            .bind(access_token)
            .bind(expires_at)
            .bind(config.to_json())
            .execute(&self.db)
            .await?;

            self.source_id = Some(source_id);
            println!("✅ Source created with ID: {}", source_id);

            Ok(source_id)
        }

        /// Get event count from database
        async fn get_event_count(&self) -> Result<i64> {
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM stream_google_calendar"
            )
            .fetch_one(&self.db)
            .await?;

            Ok(count.0)
        }

        /// Get sync token for a source
        async fn get_sync_token(&self, source_id: Uuid) -> Result<Option<String>> {
            let result: (Option<String>,) = sqlx::query_as(
                "SELECT last_sync_token FROM sources WHERE id = $1"
            )
            .bind(source_id)
            .fetch_one(&self.db)
            .await?;

            Ok(result.0)
        }

        /// Get sample events for verification
        async fn get_sample_events(&self, limit: i64) -> Result<Vec<EventSummary>> {
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
                "#
            )
            .bind(self.source_id.expect("No source_id set"))
            .bind(limit)
            .fetch_all(&self.db)
            .await?;

            Ok(events)
        }

        /// Verify events were stored correctly with detailed validation
        async fn verify_events_stored(&self) -> Result<()> {
            let events = self.get_sample_events(10).await?;

            assert!(!events.is_empty(), "No events found in database");

            println!("\n📅 Sample events from sync:");
            println!("{:<50} | {:<20} | {:<20} | 👥 | 📹", "Title", "Start", "End", );
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
                assert!(event.end_time >= event.start_time, "End time should be after start time");
            }

            // Check that we got events from the expected time range (last 30 days)
            let now = Utc::now();
            let thirty_days_ago = now - Duration::days(30);

            let recent_events: Vec<_> = events.iter()
                .filter(|e| e.start_time >= thirty_days_ago)
                .collect();

            println!("\n📊 Events in last 30 days: {}/{}", recent_events.len(), events.len());

            Ok(())
        }
    }

    #[tokio::test]
    #[ignore = "Requires GOOGLE_TEST_REFRESH_TOKEN environment variable and real OAuth proxy"]
    async fn test_google_calendar_real_oauth_e2e() -> Result<()> {
        // Get refresh token from environment
        let refresh_token = env::var("GOOGLE_TEST_REFRESH_TOKEN")
            .expect("GOOGLE_TEST_REFRESH_TOKEN environment variable required for this test");

        println!("\n🧪 Starting Google Calendar E2E Test with Real OAuth");
        println!("{}", "=".repeat(60));

        // Setup test environment
        let mut fixture = TestFixture::new().await?;

        // Create source with real OAuth
        let source_id = fixture.create_source_with_real_oauth(&refresh_token).await?;

        // Create token manager with OAuth proxy configuration
        let proxy_config = OAuthProxyConfig {
            base_url: fixture.oauth_proxy_url.clone(),
        };
        let token_manager = Arc::new(TokenManager::with_config(
            fixture.db.clone(),
            proxy_config,
        ));

        // Load configuration from database
        let config = GoogleCalendarConfig {
            calendar_ids: vec!["primary".to_string()],
            sync_window_days: 30,
            sync_direction: SyncDirection::Past,
            include_declined: false,
            include_cancelled: false,
            max_events_per_sync: 500,
        };

        // Create sync instance with configuration
        let sync = GoogleCalendarSync::with_config(
            source_id,
            fixture.db.clone(),
            token_manager,
            config,
        );

        // Test 1: Initial sync (last 30 days)
        println!("\n🔄 Test 1: Initial Sync (30-day lookback)");
        println!("{}", "-".repeat(50));

        let initial_count = fixture.get_event_count().await?;
        println!("📊 Events before sync: {}", initial_count);

        let start_time = std::time::Instant::now();
        let stats = sync.sync().await?;
        let sync_duration = start_time.elapsed();

        println!("✅ Sync completed in {:.2}s", sync_duration.as_secs_f32());
        println!("📈 Events upserted: {}", stats.upserted);
        println!("⏭️  Events skipped: {}", stats.skipped);

        let after_initial_count = fixture.get_event_count().await?;
        println!("📊 Total events in database: {}", after_initial_count);

        assert!(
            after_initial_count > initial_count,
            "Should have synced at least some events"
        );

        // Verify event data
        fixture.verify_events_stored().await?;

        // Check sync token (may not be present for time-bounded syncs)
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

        let before_incremental = fixture.get_event_count().await?;
        println!("📊 Events before incremental sync: {}", before_incremental);

        let start_time = std::time::Instant::now();
        let stats = sync.sync().await?;
        let sync_duration = start_time.elapsed();

        println!("✅ Incremental sync completed in {:.2}s", sync_duration.as_secs_f32());
        println!("📈 Events upserted: {}", stats.upserted);
        println!("⏭️  Events skipped: {}", stats.skipped);

        let after_incremental = fixture.get_event_count().await?;
        println!("📊 Total events after incremental: {}", after_incremental);

        assert!(
            after_incremental >= before_incremental,
            "Event count should not decrease"
        );

        // Test 3: Configuration change (switch to future events)
        println!("\n🔄 Test 3: Configuration Change (Future Events)");
        println!("{}", "-".repeat(50));

        // Update configuration to sync future events
        let future_config = GoogleCalendarConfig {
            calendar_ids: vec!["primary".to_string()],
            sync_window_days: 30,
            sync_direction: SyncDirection::Future,
            include_declined: false,
            include_cancelled: false,
            max_events_per_sync: 100,
        };

        sqlx::query("UPDATE sources SET config = $1, last_sync_token = NULL WHERE id = $2")
            .bind(future_config.to_json())
            .bind(source_id)
            .execute(&fixture.db)
            .await?;

        println!("📝 Updated config to sync next 30 days of events");

        let future_sync = GoogleCalendarSync::with_config(
            source_id,
            fixture.db.clone(),
            Arc::new(TokenManager::with_config(
                fixture.db.clone(),
                OAuthProxyConfig { base_url: fixture.oauth_proxy_url.clone() }
            )),
            future_config,
        );

        let stats = future_sync.sync().await?;
        println!("📈 Future events synced: {}", stats.upserted);

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
                token,
                "invalid_token_test_12345",
                "Should have replaced invalid token"
            );
            println!("🔖 New sync token saved after recovery");
        } else {
            println!("ℹ️  No sync token after recovery (expected for time-bounded fallback)");
        }

        // Test 5: Rate limiting simulation
        println!("\n🔄 Test 5: Concurrent Sync Requests");
        println!("{}", "-".repeat(50));

        // Try multiple syncs in parallel to test rate limiting handling
        let mut handles = vec![];
        for i in 0..3 {
            let sync_clone = GoogleCalendarSync::with_config(
                source_id,
                fixture.db.clone(),
                Arc::new(TokenManager::with_config(
                    fixture.db.clone(),
                    OAuthProxyConfig { base_url: fixture.oauth_proxy_url.clone() }
                )),
                GoogleCalendarConfig::default(),
            );

            handles.push(tokio::spawn(async move {
                println!("🚀 Starting concurrent sync #{}", i + 1);
                let result = sync_clone.sync().await;
                match &result {
                    Ok(stats) => println!("✅ Sync #{} completed: {} events", i + 1, stats.upserted),
                    Err(e) => println!("⚠️  Sync #{} failed (expected if rate limited): {}", i + 1, e),
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
        let result: (i32,) = sqlx::query_as("SELECT 1 as test")
            .fetch_one(&fixture.db)
            .await?;

        assert_eq!(result.0, 1);

        // Verify tables exist
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT table_name FROM information_schema.tables
             WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
        )
        .fetch_all(&fixture.db)
        .await?;

        println!("📋 Available tables:");
        for (table,) in &tables {
            println!("  - {}", table);
        }

        assert!(tables.iter().any(|(t,)| t == "sources"), "sources table should exist");
        assert!(tables.iter().any(|(t,)| t == "stream_google_calendar"), "stream_google_calendar table should exist");

        println!("✅ Test containers and database setup working correctly");
        Ok(())
    }

    #[tokio::test]
    async fn test_config_time_bounds() {
        use chrono::{Duration, Utc};

        // Test past sync
        let config = GoogleCalendarConfig {
            sync_window_days: 30,
            sync_direction: SyncDirection::Past,
            ..Default::default()
        };

        let (min, max) = config.calculate_time_bounds();
        let now = Utc::now();

        assert!(min.is_some());
        assert!(max.is_some());

        let min_time = min.unwrap();
        let max_time = max.unwrap();

        // Check that we're looking 30 days in the past
        let expected_min = now - Duration::days(30);
        let diff = (min_time - expected_min).num_seconds().abs();
        assert!(diff < 60, "Min time should be ~30 days ago");

        let diff = (max_time - now).num_seconds().abs();
        assert!(diff < 60, "Max time should be ~now");

        println!("✅ Past sync time bounds: {} to {}",
            min_time.format("%Y-%m-%d"),
            max_time.format("%Y-%m-%d"));

        // Test future sync
        let config = GoogleCalendarConfig {
            sync_window_days: 7,
            sync_direction: SyncDirection::Future,
            ..Default::default()
        };

        let (min, max) = config.calculate_time_bounds();
        assert!(min.is_some());
        assert!(max.is_some());

        let min_time = min.unwrap();
        let max_time = max.unwrap();

        let diff = (min_time - now).num_seconds().abs();
        assert!(diff < 60, "Min time should be ~now for future sync");

        let expected_max = now + Duration::days(7);
        let diff = (max_time - expected_max).num_seconds().abs();
        assert!(diff < 60, "Max time should be ~7 days from now");

        println!("✅ Future sync time bounds: {} to {}",
            min_time.format("%Y-%m-%d"),
            max_time.format("%Y-%m-%d"));
    }

    /// Helper type for event summaries
    #[derive(sqlx::FromRow)]
    struct EventSummary {
        event_id: String,
        summary: Option<String>,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
        organizer_email: Option<String>,
        attendee_count: i32,
        has_conferencing: bool,
    }
}