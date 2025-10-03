//! Database module for PostgreSQL operations

use std::collections::HashMap;

use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions, Row, Column};

use crate::error::{Error, Result};

/// Database connection and operations
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection
    pub fn new(postgres_url: &str) -> Result<Self> {
        // Pool will be created on first use
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect_lazy(postgres_url)?;

        Ok(Self { pool })
    }

    /// Initialize database (run migrations, etc.)
    pub async fn initialize(&self) -> Result<()> {
        // Test connection
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(format!("Failed to connect: {}", e)))?;

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
            .map_err(|e| Error::Database(format!("Failed to run migrations: {}", e)))?;

        Ok(())
    }

    /// Execute a query and return results
    pub async fn query(&self, sql: &str) -> Result<Vec<HashMap<String, Value>>> {
        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::new();

        for row in rows {
            let mut map = HashMap::new();

            // Get column names from the row
            for (i, column) in row.columns().iter().enumerate() {
                let name = column.name();

                // Try to get value as JSON
                let value = if let Ok(v) = row.try_get::<Value, _>(i) {
                    v
                } else if let Ok(v) = row.try_get::<String, _>(i) {
                    Value::String(v)
                } else if let Ok(v) = row.try_get::<i32, _>(i) {
                    Value::Number(v.into())
                } else if let Ok(v) = row.try_get::<i64, _>(i) {
                    Value::Number(v.into())
                } else if let Ok(v) = row.try_get::<bool, _>(i) {
                    Value::Bool(v)
                } else {
                    Value::Null
                };

                map.insert(name.to_string(), value);
            }

            results.push(map);
        }

        Ok(results)
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

    /// Health check
    pub async fn health_check(&self) -> Result<HealthStatus> {
        match sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
        {
            Ok(_) => Ok(HealthStatus {
                is_healthy: true,
                message: "Connected".to_string(),
            }),
            Err(e) => Ok(HealthStatus {
                is_healthy: false,
                message: format!("Connection failed: {}", e),
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

    #[test]
    fn test_database_creation() {
        let result = Database::new("postgresql://localhost/test");
        assert!(result.is_ok());
    }
}