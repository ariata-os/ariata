//! Database module for PostgreSQL operations

use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::error::{Error, Result};

/// Database connection and operations
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection
    pub fn new(postgres_url: &str) -> Result<Self> {
        // Get max connections from environment (default: 10)
        let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(10);

        tracing::info!("Database pool max connections: {}", max_connections);

        // Pool will be created on first use
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    // Set search_path to match migration schema configuration
                    sqlx::query("SET search_path TO data, app, public")
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
            .connect_lazy(postgres_url)?;

        Ok(Self { pool })
    }

    /// Create from an existing pool
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get the underlying connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Initialize database (run migrations, etc.)
    pub async fn initialize(&self) -> Result<()> {
        // Test connection
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to connect: {e}")))?;

        // Run migrations
        self.run_migrations().await?;

        Ok(())
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        // Use sqlx migrate to run migrations from the migrations folder
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to run migrations: {e}")))?;

        Ok(())
    }

    /// Execute a query with parameters
    pub async fn execute(&self, sql: &str, params: &[&str]) -> Result<()> {
        let mut query = sqlx::query(sql);

        for param in params {
            query = query.bind(param);
        }

        query.execute(&self.pool).await?;

        Ok(())
    }

    /// Batch insert helper - builds multi-row INSERT query with proper parameter binding
    ///
    /// This is a helper that returns the SQL query string with placeholders.
    /// Individual transforms should use this to build their batch insert queries.
    ///
    /// # Arguments
    /// * `table` - Table name (e.g., "data.location_point")
    /// * `columns` - Column names in order
    /// * `conflict_column` - Column for ON CONFLICT DO NOTHING (e.g., "source_stream_id")
    /// * `num_rows` - Number of rows to insert in this batch
    ///
    /// # Returns
    /// SQL query string with placeholders ($1, $2, $3, ...)
    ///
    /// # Example
    /// ```ignore
    /// let query_str = db.build_batch_insert_query(
    ///     "data.location_point",
    ///     &["coordinates", "latitude", "longitude"],
    ///     "source_stream_id",
    ///     100, // 100 rows
    /// );
    /// // Returns: INSERT INTO data.location_point (coordinates, latitude, longitude)
    /// //          VALUES ($1, $2, $3), ($4, $5, $6), ...
    /// //          ON CONFLICT (source_stream_id) DO NOTHING
    /// ```
    pub fn build_batch_insert_query(
        table: &str,
        columns: &[&str],
        conflict_column: &str,
        num_rows: usize,
    ) -> String {
        let num_cols = columns.len();

        let mut query = format!("INSERT INTO {} (", table);
        query.push_str(&columns.join(", "));
        query.push_str(") VALUES ");

        // Build VALUES clauses: ($1, $2, $3), ($4, $5, $6), ...
        let mut value_clauses = Vec::with_capacity(num_rows);
        for row_idx in 0..num_rows {
            let mut placeholders = Vec::with_capacity(num_cols);
            for col_idx in 0..num_cols {
                let param_num = row_idx * num_cols + col_idx + 1;
                placeholders.push(format!("${}", param_num));
            }
            value_clauses.push(format!("({})", placeholders.join(", ")));
        }

        query.push_str(&value_clauses.join(", "));
        query.push_str(&format!(" ON CONFLICT ({}) DO NOTHING", conflict_column));

        query
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthStatus> {
        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => Ok(HealthStatus {
                is_healthy: true,
                message: "Connected".to_string(),
            }),
            Err(e) => Ok(HealthStatus {
                is_healthy: false,
                message: format!("Connection failed: {e}"),
            }),
        }
    }
}

/// Health status for database
#[derive(Debug)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_creation() {
        let result = Database::new("postgresql://localhost/test");
        assert!(result.is_ok());
    }
}
