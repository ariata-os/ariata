//! Test fixture that manages containers and database setup for integration tests

use ariata::error::Result;
use chrono::{Duration, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;
use testcontainers_modules::testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::{postgres::Postgres, minio::MinIO};
use uuid::Uuid;

/// Test fixture that manages containers and database setup
pub struct TestFixture {
    pub db: PgPool,
    _pg_container: ContainerAsync<Postgres>,
    _minio_container: ContainerAsync<MinIO>,
    pub source_id: Option<Uuid>,
    pub oauth_proxy_url: String,
}

impl TestFixture {
    /// Create a new test fixture with running containers
    pub async fn new() -> Result<Self> {
        // Load environment variables from root .env file (for E2E tests)
        // Try multiple possible paths since working directory may vary
        match dotenv::from_path("../.env") {
            Ok(_) => println!("✅ Loaded environment from ../.env"),
            Err(_) => {
                dotenv::from_path(".env").ok();
                println!("✅ Loaded environment from .env (or using system env)");
            }
        }

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

    /// Create a Google source with OAuth tokens using the real OAuth proxy
    pub async fn create_google_source_with_oauth(
        &mut self,
        refresh_token: &str,
        config: serde_json::Value,
    ) -> Result<Uuid> {
        println!("\n📝 Creating Google source with real OAuth...");

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
            .map_err(|e| ariata::error::Error::Network(format!("OAuth proxy request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ariata::error::Error::Authentication(
                format!("OAuth token refresh failed: {}", error_text)
            ));
        }

        let token_response: serde_json::Value = response.json().await
            .map_err(|e| ariata::error::Error::Other(format!("Failed to parse token response: {}", e)))?;

        let access_token = token_response["access_token"].as_str()
            .ok_or_else(|| ariata::error::Error::Other("Missing access token in response".to_string()))?;

        let expires_in = token_response["expires_in"].as_i64().unwrap_or(3600);
        let expires_at = Utc::now() + Duration::seconds(expires_in);

        println!("✅ Got access token (expires in {} seconds)", expires_in);

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
        .bind("Test Google Source")
        .bind(true)
        .bind(refresh_token)
        .bind(access_token)
        .bind(expires_at)
        .bind(config)
        .execute(&self.db)
        .await?;

        self.source_id = Some(source_id);
        println!("✅ Source created with ID: {}", source_id);

        Ok(source_id)
    }

    /// Get sync token for a source
    pub async fn get_sync_token(&self, source_id: Uuid) -> Result<Option<String>> {
        let result: (Option<String>,) = sqlx::query_as(
            "SELECT last_sync_token FROM sources WHERE id = $1"
        )
        .bind(source_id)
        .fetch_one(&self.db)
        .await?;

        Ok(result.0)
    }

    /// Verify database connection
    pub async fn verify_connection(&self) -> Result<()> {
        let result: (i32,) = sqlx::query_as("SELECT 1 as test")
            .fetch_one(&self.db)
            .await?;

        assert_eq!(result.0, 1);
        Ok(())
    }

    /// Get list of all tables in the database
    pub async fn get_tables(&self) -> Result<Vec<String>> {
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT table_name FROM information_schema.tables
             WHERE table_schema = 'public' AND table_type = 'BASE TABLE'
             ORDER BY table_name"
        )
        .fetch_all(&self.db)
        .await?;

        Ok(tables.into_iter().map(|(name,)| name).collect())
    }
}
