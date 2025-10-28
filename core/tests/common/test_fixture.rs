//! Test fixture that manages containers and database setup for integration tests

use ariata::error::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};
use testcontainers_modules::testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::{postgres::Postgres, minio::MinIO};

/// Test fixture that manages containers and database setup
pub struct TestFixture {
    pub db: PgPool,
    _pg_container: ContainerAsync<Postgres>,
    _minio_container: ContainerAsync<MinIO>,
}

impl TestFixture {
    /// Create a new test fixture with running containers
    pub async fn new() -> Result<Self> {
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

        // Start MinIO container for S3-compatible storage
        let minio_container = MinIO::default()
            .start()
            .await
            .expect("MinIO container failed to start");

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
        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .expect("Failed to run migrations");

        Ok(Self {
            db,
            _pg_container: pg_container,
            _minio_container: minio_container,
        })
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
