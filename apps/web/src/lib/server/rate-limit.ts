/**
 * API rate limiting and usage tracking
 *
 * Hardcoded rate limits enforced per-instance (not per-user).
 * In an instance-per-user deployment model, each user gets their own database,
 * so these limits apply to each user's instance independently.
 */

import { getPool } from './db';
import { env } from '$env/dynamic/private';
import type { Pool } from 'pg';

export class RateLimitError extends Error {
	constructor(
		message: string,
		public readonly current: number,
		public readonly limit: number,
		public readonly resetAt?: Date
	) {
		super(message);
		this.name = 'RateLimitError';
	}
}

/**
 * Rate limits applied to all instances
 * Configurable via environment variables with safe defaults
 */
export interface RateLimits {
	chatRequestsPerDay: number;
	chatTokensPerDay: number;
	backgroundJobsPerDay: number;
}

/**
 * Parse environment variable with validation
 * @param key Environment variable name
 * @param defaultValue Default value if not set
 * @param min Minimum allowed value (enforces safety)
 */
function parseEnvInt(key: string, defaultValue: number, min: number): number {
	const value = env[key];
	if (!value) return defaultValue;

	const parsed = parseInt(value, 10);
	if (isNaN(parsed)) {
		console.warn(`Invalid value for ${key}: "${value}", using default ${defaultValue}`);
		return defaultValue;
	}

	// Enforce minimum to prevent disabling limits
	if (parsed < min) {
		console.warn(
			`Value for ${key} (${parsed}) is below minimum (${min}), using minimum instead`
		);
		return min;
	}

	return parsed;
}

export const DEFAULT_RATE_LIMITS: RateLimits = {
	chatRequestsPerDay: parseEnvInt('RATE_LIMIT_CHAT_DAILY', 1000, 1),
	chatTokensPerDay: parseEnvInt('RATE_LIMIT_TOKENS_DAILY', 500_000, 1000),
	backgroundJobsPerDay: parseEnvInt('RATE_LIMIT_JOBS_DAILY', 100, 1)
};

export interface TokenUsage {
	input: number;
	output: number;
	model: string;
}

/**
 * Check if an endpoint request would exceed rate limits
 *
 * Atomically increments the request count and then checks if limits are exceeded.
 * Uses optimistic locking: if limit is exceeded after increment, the request has
 * already been counted. This is acceptable for daily limits as it only allows
 * one extra request rather than unbounded concurrent bypass.
 *
 * Uses server-side timestamps (UTC) to prevent client tampering.
 *
 * @throws {RateLimitError} if limits would be exceeded
 */
export async function checkRateLimit(
	endpoint: string,
	limits: RateLimits = DEFAULT_RATE_LIMITS
): Promise<void> {
	const pool = getPool();
	const now = new Date();
	const dayBucket = getDayBucket(now);

	// Get the limit for this endpoint
	const dailyRequestLimit =
		endpoint === 'chat'
			? limits.chatRequestsPerDay
			: endpoint === 'background_job'
				? limits.backgroundJobsPerDay
				: Number.MAX_SAFE_INTEGER;

	// Atomically increment request count and get new total
	// This prevents race conditions by using database-level atomicity
	const result = await pool.query(
		`
		INSERT INTO api_usage
			(endpoint, day_bucket, request_count)
		VALUES ($1, $2, 1)
		ON CONFLICT (endpoint, day_bucket)
		DO UPDATE SET
			request_count = api_usage.request_count + 1,
			updated_at = NOW()
		RETURNING request_count
		`,
		[endpoint, dayBucket]
	);

	const newCount = result.rows[0]?.request_count || 1;

	// Check if we've exceeded the limit (after incrementing)
	if (newCount > dailyRequestLimit) {
		// Calculate reset time (next day at midnight UTC)
		const resetAt = new Date(dayBucket + 'T00:00:00.000Z');
		resetAt.setUTCDate(resetAt.getUTCDate() + 1);
		throw new RateLimitError(
			`Daily rate limit exceeded: ${newCount}/${dailyRequestLimit} requests. Resets at ${resetAt.toISOString()}`,
			newCount,
			dailyRequestLimit,
			resetAt
		);
	}

	// Check daily token limit (applies to all endpoints combined)
	const totalDailyTokens = await getTotalDailyTokens(pool, dayBucket);
	if (totalDailyTokens > limits.chatTokensPerDay) {
		// Calculate reset time (next day at midnight UTC)
		const resetAt = new Date(dayBucket + 'T00:00:00.000Z');
		resetAt.setUTCDate(resetAt.getUTCDate() + 1);
		throw new RateLimitError(
			`Daily token limit exceeded: ${totalDailyTokens}/${limits.chatTokensPerDay} tokens. Resets at ${resetAt.toISOString()}`,
			totalDailyTokens,
			limits.chatTokensPerDay,
			resetAt
		);
	}
}

/**
 * Record token usage after a successful API call
 *
 * IMPORTANT: This function MUST be called after checkRateLimit() to ensure
 * the api_usage row exists. Calling this without checkRateLimit() will throw an error.
 *
 * Note: Request count is already incremented by checkRateLimit().
 * This function only updates token counts and cost estimates.
 *
 * @throws {Error} if no row exists (checkRateLimit not called first)
 * @throws {Error} if token counts exceed safe integer range
 */
export async function recordUsage(endpoint: string, tokens: TokenUsage): Promise<void> {
	const pool = getPool();
	const now = new Date();
	const dayBucket = getDayBucket(now);
	const cost = calculateCost(tokens);

	// Validate token counts don't exceed safe integer range
	// PostgreSQL INTEGER max is 2,147,483,647
	// JavaScript MAX_SAFE_INTEGER is 9,007,199,254,740,991
	const totalTokens = tokens.input + tokens.output;
	if (
		totalTokens > Number.MAX_SAFE_INTEGER ||
		totalTokens > 2147483647 ||
		tokens.input > 2147483647 ||
		tokens.output > 2147483647
	) {
		throw new Error(
			`Token count exceeds safe range: input=${tokens.input}, output=${tokens.output}, total=${totalTokens}`
		);
	}

	// Update token usage (request count already incremented by checkRateLimit)
	const result = await pool.query(
		`
		UPDATE api_usage
		SET
			token_count = token_count + $3,
			input_tokens = input_tokens + $4,
			output_tokens = output_tokens + $5,
			estimated_cost_usd = estimated_cost_usd + $6,
			updated_at = NOW()
		WHERE endpoint = $1 AND day_bucket = $2
		`,
		[endpoint, dayBucket, totalTokens, tokens.input, tokens.output, cost]
	);

	// Verify the UPDATE affected at least one row
	// If no rows were affected, it means checkRateLimit() wasn't called first
	if (result.rowCount === 0) {
		throw new Error(
			`Failed to record usage: no row found for endpoint="${endpoint}", day="${dayBucket}". ` +
				`Ensure checkRateLimit() is called before recordUsage().`
		);
	}
}

/**
 * Get current usage statistics for display in UI
 */
export interface UsageStats {
	dailyRequests: number;
	dailyTokens: number;
	dailyCost: number;
	limits: RateLimits;
}

export async function getUsageStats(endpoint: string): Promise<UsageStats> {
	const pool = getPool();
	const now = new Date();
	const dayBucket = getDayBucket(now);
	const limits = DEFAULT_RATE_LIMITS;

	const [dailyUsage, totalDailyTokens, dailyCost] = await Promise.all([
		getDailyUsage(pool, endpoint, dayBucket),
		getTotalDailyTokens(pool, dayBucket),
		getDailyCost(pool, dayBucket)
	]);

	return {
		dailyRequests: dailyUsage.requestCount,
		dailyTokens: totalDailyTokens,
		dailyCost,
		limits
	};
}

// ============================================================================
// Helper Functions
// ============================================================================

function getDayBucket(date: Date): string {
	const d = new Date(date);
	d.setUTCHours(0, 0, 0, 0);
	// Return date as string in YYYY-MM-DD format to match PostgreSQL DATE type
	// This matches Rust's NaiveDate representation
	return d.toISOString().split('T')[0];
}

interface DailyUsage {
	requestCount: number;
	tokenCount: number;
}

async function getDailyUsage(pool: Pool, endpoint: string, dayBucket: string): Promise<DailyUsage> {
	const result = await pool.query(
		`
		SELECT
			COALESCE(request_count, 0) as request_count,
			COALESCE(token_count, 0) as token_count
		FROM api_usage
		WHERE endpoint = $1 AND day_bucket = $2
		`,
		[endpoint, dayBucket]
	);

	return {
		requestCount: result.rows[0]?.request_count ?? 0,
		tokenCount: result.rows[0]?.token_count ?? 0
	};
}

async function getTotalDailyTokens(pool: Pool, dayBucket: string): Promise<number> {
	const result = await pool.query(
		`
		SELECT COALESCE(SUM(token_count), 0)::int as total_tokens
		FROM api_usage
		WHERE day_bucket = $1
		`,
		[dayBucket]
	);

	return result.rows[0]?.total_tokens ?? 0;
}

async function getDailyCost(pool: Pool, dayBucket: string): Promise<number> {
	const result = await pool.query(
		`
		SELECT COALESCE(SUM(estimated_cost_usd), 0)::numeric as total_cost
		FROM api_usage
		WHERE day_bucket = $1
		`,
		[dayBucket]
	);

	return parseFloat(result.rows[0]?.total_cost ?? 0);
}

/**
 * Calculate estimated cost based on token usage and model
 * Pricing as of November 2024 (subject to change)
 */
function calculateCost(tokens: TokenUsage): number {
	const modelLower = tokens.model.toLowerCase();

	// Anthropic Claude pricing (per million tokens)
	let inputPrice = 3.0; // Default to Sonnet
	let outputPrice = 15.0;

	if (modelLower.includes('sonnet')) {
		if (modelLower.includes('4')) {
			inputPrice = 3.0;
			outputPrice = 15.0;
		} else {
			inputPrice = 3.0;
			outputPrice = 15.0;
		}
	} else if (modelLower.includes('haiku')) {
		inputPrice = 0.8;
		outputPrice = 4.0;
	} else if (modelLower.includes('opus')) {
		inputPrice = 15.0;
		outputPrice = 75.0;
	} else if (modelLower.includes('gpt')) {
		// OpenAI pricing estimates
		if (modelLower.includes('gpt-5')) {
			inputPrice = 5.0;
			outputPrice = 15.0;
		} else if (modelLower.includes('oss')) {
			inputPrice = 0.0; // Open source models via AI Gateway
			outputPrice = 0.0;
		} else {
			inputPrice = 2.5;
			outputPrice = 10.0;
		}
	} else if (modelLower.includes('gemini')) {
		inputPrice = 1.25;
		outputPrice = 5.0;
	}

	const inputCost = (tokens.input * inputPrice) / 1_000_000;
	const outputCost = (tokens.output * outputPrice) / 1_000_000;

	return inputCost + outputCost;
}
