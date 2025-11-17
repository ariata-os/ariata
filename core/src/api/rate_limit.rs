//! Rate limiting and usage tracking for API endpoints
//!
//! Provides hardcoded rate limits that are enforced per-instance (not per-user).
//! In an instance-per-user deployment model, each user gets their own database,
//! so these limits apply to each user's instance independently.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::types::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("Daily rate limit exceeded: {current}/{limit} requests. Resets at {reset_at}")]
    DailyLimitExceeded {
        current: i32,
        limit: i32,
        reset_at: String,
    },

    #[error("Daily token limit exceeded: {current}/{limit} tokens. Resets at {reset_at}")]
    TokenLimitExceeded {
        current: i32,
        limit: i32,
        reset_at: String,
    },

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// Rate limits applied to all instances
/// In instance-per-user model, each user gets these same limits
/// Configurable via environment variables with safe defaults
#[derive(Debug, Clone)]
pub struct RateLimits {
    /// Maximum chat requests per day
    pub chat_requests_per_day: i32,
    /// Maximum tokens (input + output) per day across all endpoints
    pub chat_tokens_per_day: i32,
    /// Maximum background job LLM calls per day
    pub background_jobs_per_day: i32,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            chat_requests_per_day: 1000,       // 1000 chat requests/day
            chat_tokens_per_day: 500_000,      // 500K tokens/day (~$1.50/day worst case)
            background_jobs_per_day: 100,      // 100 background LLM jobs/day
        }
    }
}

impl RateLimits {
    /// Load rate limits from environment variables with safe defaults
    ///
    /// Environment variables:
    /// - RATE_LIMIT_CHAT_DAILY (default: 1000, min: 1)
    /// - RATE_LIMIT_TOKENS_DAILY (default: 500000, min: 1000)
    /// - RATE_LIMIT_JOBS_DAILY (default: 100, min: 1)
    pub fn from_env() -> Self {
        fn parse_env(key: &str, default: i32, min: i32) -> i32 {
            std::env::var(key)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(default)
                .max(min) // Enforce minimum to prevent disabling limits
        }

        Self {
            chat_requests_per_day: parse_env("RATE_LIMIT_CHAT_DAILY", 1000, 1),
            chat_tokens_per_day: parse_env("RATE_LIMIT_TOKENS_DAILY", 500_000, 1000),
            background_jobs_per_day: parse_env("RATE_LIMIT_JOBS_DAILY", 100, 1),
        }
    }

    /// Create custom rate limits (useful for testing or custom tiers)
    pub fn custom(
        chat_per_day: i32,
        tokens_per_day: i32,
        jobs_per_day: i32,
    ) -> Self {
        Self {
            chat_requests_per_day: chat_per_day,
            chat_tokens_per_day: tokens_per_day,
            background_jobs_per_day: jobs_per_day,
        }
    }
}

/// Token usage information
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input: u32,
    pub output: u32,
    pub model: String,
}

impl TokenUsage {
    pub fn new(input: u32, output: u32, model: impl Into<String>) -> Self {
        Self {
            input,
            output,
            model: model.into(),
        }
    }

    pub fn total(&self) -> u32 {
        self.input + self.output
    }
}

/// Check if an endpoint request would exceed rate limits
///
/// Atomically increments the request count and then checks if limits are exceeded.
/// Uses optimistic locking: if limit is exceeded after increment, the request has
/// already been counted. This is acceptable for daily limits as it only allows
/// one extra request rather than unbounded concurrent bypass.
///
/// Uses server-side timestamps (UTC) to prevent client tampering.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `endpoint` - Endpoint identifier (e.g., "chat", "background_job")
/// * `limits` - Rate limit configuration
///
/// # Returns
/// * `Ok(())` if within limits
/// * `Err(RateLimitError)` if limits would be exceeded
pub async fn check_rate_limit(
    pool: &PgPool,
    endpoint: &str,
    limits: &RateLimits,
) -> Result<(), RateLimitError> {
    let now = Utc::now();
    let day_bucket = get_day_bucket(now);

    // Get the limit for this endpoint
    let daily_request_limit = match endpoint {
        "chat" => limits.chat_requests_per_day,
        "background_job" => limits.background_jobs_per_day,
        _ => i32::MAX, // No limit for other endpoints
    };

    // Atomically increment request count and get new total
    // This prevents race conditions by using database-level atomicity
    let new_count = sqlx::query!(
        r#"
        INSERT INTO app.api_usage
            (endpoint, day_bucket, request_count)
        VALUES ($1, $2, 1)
        ON CONFLICT (endpoint, day_bucket)
        DO UPDATE SET
            request_count = api_usage.request_count + 1,
            updated_at = NOW()
        RETURNING request_count as "count!"
        "#,
        endpoint,
        day_bucket
    )
    .fetch_one(pool)
    .await?
    .count;

    // Check if we've exceeded the limit (after incrementing)
    if new_count > daily_request_limit {
        // Calculate reset time (next day at midnight UTC)
        let reset_at = day_bucket
            .succ_opt()
            .unwrap_or(day_bucket)
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .to_rfc3339();

        return Err(RateLimitError::DailyLimitExceeded {
            current: new_count,
            limit: daily_request_limit,
            reset_at,
        });
    }

    // Check daily token limit (applies to all endpoints combined)
    let total_daily_tokens = get_total_daily_tokens(pool, day_bucket).await?;
    if total_daily_tokens > limits.chat_tokens_per_day {
        // Calculate reset time (next day at midnight UTC)
        let reset_at = day_bucket
            .succ_opt()
            .unwrap_or(day_bucket)
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .to_rfc3339();

        return Err(RateLimitError::TokenLimitExceeded {
            current: total_daily_tokens,
            limit: limits.chat_tokens_per_day,
            reset_at,
        });
    }

    Ok(())
}

/// Record token usage after a successful API call
///
/// **IMPORTANT**: This function MUST be called after `check_rate_limit()` to ensure
/// the `api_usage` row exists. Calling this without `check_rate_limit()` will return an error.
///
/// Note: Request count is already incremented by `check_rate_limit()`.
/// This function only updates token counts and cost estimates.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `endpoint` - Endpoint identifier
/// * `tokens` - Token usage information
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(sqlx::Error::RowNotFound)` if no row exists (check_rate_limit not called first)
/// * `Err(sqlx::Error::Protocol)` if token counts exceed i32::MAX
/// * `Err(sqlx::Error)` on other database errors
pub async fn record_usage(
    pool: &PgPool,
    endpoint: &str,
    tokens: TokenUsage,
) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    let day_bucket = get_day_bucket(now);

    let cost = calculate_cost(&tokens);

    // Convert token counts with overflow protection
    let total_tokens = i32::try_from(tokens.total())
        .map_err(|_| sqlx::Error::Protocol("Token count exceeds i32::MAX".into()))?;
    let input_tokens = i32::try_from(tokens.input)
        .map_err(|_| sqlx::Error::Protocol("Input token count exceeds i32::MAX".into()))?;
    let output_tokens = i32::try_from(tokens.output)
        .map_err(|_| sqlx::Error::Protocol("Output token count exceeds i32::MAX".into()))?;

    // Update token usage (request count already incremented by check_rate_limit)
    let result = sqlx::query!(
        r#"
        UPDATE app.api_usage
        SET
            token_count = token_count + $3,
            input_tokens = input_tokens + $4,
            output_tokens = output_tokens + $5,
            estimated_cost_usd = estimated_cost_usd + $6,
            updated_at = NOW()
        WHERE endpoint = $1 AND day_bucket = $2
        "#,
        endpoint,
        day_bucket,
        total_tokens,
        input_tokens,
        output_tokens,
        cost
    )
    .execute(pool)
    .await?;

    // Verify the UPDATE affected at least one row
    // If no rows were affected, it means check_rate_limit() wasn't called first
    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }

    Ok(())
}

/// Get current usage statistics for display in UI
#[derive(Debug, Clone)]
pub struct UsageStats {
    pub daily_requests: i32,
    pub daily_tokens: i32,
    pub daily_cost: Decimal,
    pub limits: RateLimits,
}

pub async fn get_usage_stats(pool: &PgPool, endpoint: &str) -> Result<UsageStats, sqlx::Error> {
    let now = Utc::now();
    let day_bucket = get_day_bucket(now);
    let limits = RateLimits::default();

    let daily_usage = get_daily_usage(pool, endpoint, day_bucket).await?;
    let total_daily_tokens = get_total_daily_tokens(pool, day_bucket).await?;
    let daily_cost = get_daily_cost(pool, day_bucket).await?;

    Ok(UsageStats {
        daily_requests: daily_usage.request_count,
        daily_tokens: total_daily_tokens,
        daily_cost,
        limits,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_day_bucket(dt: DateTime<Utc>) -> NaiveDate {
    dt.date_naive()
}

#[derive(Debug)]
struct DailyUsage {
    request_count: i32,
    #[allow(dead_code)]
    token_count: i32,
}

async fn get_daily_usage(
    pool: &PgPool,
    endpoint: &str,
    day_bucket: NaiveDate,
) -> Result<DailyUsage, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT
            COALESCE(request_count, 0) as "request_count!",
            COALESCE(token_count, 0) as "token_count!"
        FROM app.api_usage
        WHERE endpoint = $1 AND day_bucket = $2
        "#,
        endpoint,
        day_bucket
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| DailyUsage {
        request_count: r.request_count,
        token_count: r.token_count,
    }).unwrap_or(DailyUsage {
        request_count: 0,
        token_count: 0,
    }))
}

async fn get_total_daily_tokens(pool: &PgPool, day_bucket: NaiveDate) -> Result<i32, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT COALESCE(SUM(token_count), 0)::int as "total_tokens!"
        FROM app.api_usage
        WHERE day_bucket = $1
        "#,
        day_bucket
    )
    .fetch_one(pool)
    .await?;

    Ok(result.total_tokens)
}

async fn get_daily_cost(pool: &PgPool, day_bucket: NaiveDate) -> Result<Decimal, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT COALESCE(SUM(estimated_cost_usd), 0) as "total_cost!"
        FROM app.api_usage
        WHERE day_bucket = $1
        "#,
        day_bucket
    )
    .fetch_one(pool)
    .await?;

    Ok(result.total_cost)
}

/// Calculate estimated cost based on token usage and model
/// Pricing as of November 2024 (subject to change)
fn calculate_cost(tokens: &TokenUsage) -> Decimal {
    let model_lower = tokens.model.to_lowercase();

    // Anthropic Claude pricing (per million tokens)
    let (input_price, output_price) = if model_lower.contains("sonnet") {
        if model_lower.contains("4") {
            (Decimal::from_str("3.00").unwrap(), Decimal::from_str("15.00").unwrap()) // Claude Sonnet 4
        } else {
            (Decimal::from_str("3.00").unwrap(), Decimal::from_str("15.00").unwrap()) // Claude 3.5 Sonnet
        }
    } else if model_lower.contains("haiku") {
        (Decimal::from_str("0.80").unwrap(), Decimal::from_str("4.00").unwrap()) // Claude 3.5 Haiku
    } else if model_lower.contains("opus") {
        (Decimal::from_str("15.00").unwrap(), Decimal::from_str("75.00").unwrap()) // Claude 3 Opus
    } else {
        // Default to Sonnet pricing if unknown
        (Decimal::from_str("3.00").unwrap(), Decimal::from_str("15.00").unwrap())
    };

    let input_cost = Decimal::from(tokens.input) * input_price / Decimal::from(1_000_000);
    let output_cost = Decimal::from(tokens.output) * output_price / Decimal::from(1_000_000);

    input_cost + output_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cost_sonnet() {
        let tokens = TokenUsage::new(1000, 500, "claude-sonnet-4");
        let cost = calculate_cost(&tokens);

        // 1000 * 0.003 / 1000 + 500 * 0.015 / 1000 = 0.003 + 0.0075 = 0.0105
        assert!(cost > Decimal::from_str("0.01").unwrap());
        assert!(cost < Decimal::from_str("0.02").unwrap());
    }

    #[test]
    fn test_calculate_cost_haiku() {
        let tokens = TokenUsage::new(10000, 5000, "claude-haiku-3.5");
        let cost = calculate_cost(&tokens);

        // Should be much cheaper than Sonnet
        assert!(cost < Decimal::from_str("0.05").unwrap());
    }

    #[test]
    fn test_rate_limits_default() {
        let limits = RateLimits::default();
        assert_eq!(limits.chat_requests_per_day, 1000);
        assert_eq!(limits.chat_tokens_per_day, 500_000);
        assert_eq!(limits.background_jobs_per_day, 100);
    }

    #[test]
    fn test_token_usage_total() {
        let tokens = TokenUsage::new(1000, 500, "claude-sonnet-4");
        assert_eq!(tokens.total(), 1500);
    }
}
