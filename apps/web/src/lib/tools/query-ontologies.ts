import { tool } from 'ai';
import { z } from 'zod';
import { getPool } from '$lib/server/db';
import type { Pool } from 'pg';
import { env } from '$env/dynamic/private';

/**
 * Query which ontology tables are available based on enabled streams
 *
 * This fetches from the Rust API which uses the centralized transform registry
 * as the single source of truth for stream→ontology mappings.
 *
 * @throws Error if the Rust API is unreachable or returns an error
 */
export async function getAvailableOntologies(): Promise<Set<string>> {
	const RUST_API_URL = env.RUST_API_URL || 'http://localhost:8000';

	// Add timeout to prevent hanging
	const controller = new AbortController();
	const timeoutId = setTimeout(() => controller.abort(), 5000); // 5 second timeout

	try {
		const response = await fetch(`${RUST_API_URL}/api/ontologies/available`, {
			signal: controller.signal
		});

		clearTimeout(timeoutId);

		if (!response.ok) {
			throw new Error(
				`Failed to fetch available ontologies: ${response.status} ${response.statusText}`
			);
		}

		const ontologies: string[] = await response.json();
		return new Set(ontologies);
	} catch (error) {
		clearTimeout(timeoutId);

		if (error instanceof Error && error.name === 'AbortError') {
			throw new Error('Timeout fetching available ontologies from Rust API');
		}

		console.error('[getAvailableOntologies] Error fetching from Rust API:', error);
		throw error; // Propagate error instead of silently returning empty set
	}
}

/**
 * Build dynamic tool description based on available ontology tables
 */
function buildToolDescription(availableOntologies: Set<string>): string {
	// Full table descriptions grouped by domain
	const tableDescriptions: Record<string, string> = {
		// Entity primitives
		entities_person:
			'entities_person: canonical_name, email_addresses[], phone_numbers[], relationship_category, first_interaction, last_interaction',
		entities_place:
			'entities_place: canonical_name, category, geo_center, visit_count, total_time_minutes',
		entities_topic:
			'entities_topic: name, category, keywords[], first_mentioned, last_mentioned, mention_count',

		// Health domain
		health_heart_rate: 'health_heart_rate: bpm, measurement_context, timestamp',
		health_sleep:
			'health_sleep: total_duration_minutes, sleep_quality_score, sleep_stages (jsonb), start_time, end_time',
		health_workout:
			'health_workout: activity_type, intensity, calories_burned, average_heart_rate, distance_meters, start_time, end_time, place_id',
		health_steps: 'health_steps: step_count, timestamp',
		health_mood:
			'health_mood: valence, arousal, mood_category, measurement_method, timestamp',
		health_meal:
			'health_meal: meal_type, foods (jsonb), total_calories, protein_grams, carbs_grams, fat_grams, timestamp, place_id',

		// Location domain
		location_point:
			'location_point: coordinates (geography), latitude, longitude, altitude_meters, accuracy_meters, timestamp',
		location_visit:
			'location_visit: place_id, centroid_coordinates, latitude, longitude, start_time, end_time',

		// Social domain
		social_email:
			'social_email: message_id, thread_id, channel, body, from_identifier, to_identifiers[], direction, is_read, timestamp',
		social_message:
			'social_message: message_id, thread_id, channel, body, from_identifier, to_identifiers[], direction, is_read, timestamp',
		social_call:
			'social_call: call_type, direction, call_status, duration_seconds, start_time, end_time',
		social_interaction:
			'social_interaction: interaction_type, title, description, participant_identifiers[], start_time, end_time, place_id',
		social_post:
			'social_post: platform, post_id, post_type, content, like_count, repost_count, timestamp',

		// Activity domain
		activity_calendar_entry:
			'activity_calendar_entry: title, description, calendar_name, event_type, organizer_identifier, attendee_identifiers[], location_name (NOT "location"), place_id, conference_url, conference_platform, start_time, end_time, is_all_day (NOT "all_day"), status, response_status',
		activity_app_usage:
			'activity_app_usage: app_name, app_bundle_id, app_category, window_title, start_time, end_time',
		activity_screen_time:
			'activity_screen_time: device_name, device_type, total_screen_time_seconds, unlock_count, start_time, end_time',
		activity_web_browsing:
			'activity_web_browsing: url, domain, page_title, visit_duration_seconds, timestamp',

		// Finance domain
		finance_balance:
			'finance_balance: account_name, account_type, institution_name, balance_cents, currency, timestamp',
		finance_transaction:
			'finance_transaction: transaction_id, description, merchant_name, amount_cents, currency, category, subcategory, timestamp, place_id',
		finance_subscription:
			'finance_subscription: service_name, subscription_type, amount_cents, billing_period_days, status, next_billing_date',

		// Knowledge domain
		knowledge_document:
			'knowledge_document: title, content, content_summary, document_type, external_url, tags[], is_authored, created_time, last_modified_time',
		knowledge_bookmark:
			'knowledge_bookmark: url, title, description, page_content, tags[], saved_at',
		knowledge_search:
			'knowledge_search: query, search_engine, result_count, clicked_result_url, timestamp',

		// Speech domain
		speech_transcription:
			'speech_transcription: audio_file_path, transcript_text, language, confidence_score, speaker_count, recorded_at',

		// Introspection domain
		introspection_journal:
			'introspection_journal: title, content, sentiment_score, tags[], entry_type, entry_date',
		introspection_goal:
			'introspection_goal: title, description, goal_type, status, progress_percent, created_date, target_date, completed_date',
		introspection_gratitude:
			'introspection_gratitude: content, gratitude_category, person_ids[], place_ids[], entry_date'
	};

	// Build description with only available tables
	let description = `Query the user's life logging data from the ontology database.
This includes health metrics, social interactions, calendar events, locations, financial transactions,
and more. Use this tool to answer questions about the user's activities, habits, and patterns.

Available tables in elt schema (with key fields):

`;

	// Group tables by domain and filter to only available ones
	const domains = [
		{ name: 'ENTITY PRIMITIVES', tables: ['entities_person', 'entities_place', 'entities_topic'] },
		{
			name: 'HEALTH DOMAIN',
			tables: [
				'health_heart_rate',
				'health_sleep',
				'health_workout',
				'health_steps',
				'health_mood',
				'health_meal'
			]
		},
		{ name: 'LOCATION DOMAIN', tables: ['location_point', 'location_visit'] },
		{
			name: 'SOCIAL DOMAIN',
			tables: ['social_email', 'social_message', 'social_call', 'social_interaction', 'social_post']
		},
		{
			name: 'ACTIVITY DOMAIN',
			tables: [
				'activity_calendar_entry',
				'activity_app_usage',
				'activity_screen_time',
				'activity_web_browsing'
			]
		},
		{
			name: 'FINANCE DOMAIN',
			tables: ['finance_balance', 'finance_transaction', 'finance_subscription']
		},
		{
			name: 'KNOWLEDGE DOMAIN',
			tables: ['knowledge_document', 'knowledge_bookmark', 'knowledge_search']
		},
		{ name: 'SPEECH DOMAIN', tables: ['speech_transcription'] },
		{
			name: 'INTROSPECTION DOMAIN',
			tables: ['introspection_journal', 'introspection_goal', 'introspection_gratitude']
		}
	];

	for (const domain of domains) {
		const availableTables = domain.tables.filter((table) => availableOntologies.has(table));
		if (availableTables.length > 0) {
			description += `${domain.name}:\n`;
			for (const table of availableTables) {
				description += `- ${tableDescriptions[table]}\n`;
			}
			description += '\n';
		}
	}

	description += `All tables have: source_stream_id, source_table, source_provider, metadata (jsonb), created_at, updated_at

IMPORTANT: Use exact table names as shown - they are SINGULAR (e.g., "location_point" NOT "location_points").
Use proper field names exactly as shown above.`;

	return description;
}

/**
 * Create a query ontologies tool with dynamic description based on available data
 */
export async function createQueryOntologiesTool(pool: Pool) {
	const availableOntologies = await getAvailableOntologies();
	const description = buildToolDescription(availableOntologies);

	return tool({
		description,
		inputSchema: z.object({
			query: z
				.string()
				.describe(
					'The PostgreSQL SELECT query to execute. Must be read-only (SELECT only). Use ILIKE for case-insensitive string matching. All tables are in the elt schema. CRITICAL: Use exact table names - they are SINGULAR (e.g., location_point NOT location_points).'
				),
			reasoning: z
				.string()
				.describe(
					"Brief explanation of what you're trying to find and why this query will answer the user's question"
				)
		}),

		execute: async ({ query, reasoning }) => {
			console.log('[queryOntologiesTool] Reasoning:', reasoning);
			console.log('[queryOntologiesTool] Query:', query);

			// Security: Ensure query is read-only
			const normalizedQuery = query.trim().toLowerCase();

			// Use word boundary regex to match SQL keywords only (not substrings in column names like 'created_at')
			const forbiddenPatterns = [
				/\binsert\b/i,
				/\bupdate\b/i,
				/\bdelete\b/i,
				/\bdrop\b/i,
				/\btruncate\b/i,
				/\balter\b/i,
				/\bcreate\b/i
			];

			const hasForbiddenKeyword = forbiddenPatterns.some(pattern => pattern.test(normalizedQuery));

			if (!normalizedQuery.startsWith('select') || hasForbiddenKeyword) {
				return {
					success: false,
					error: 'Only SELECT queries are allowed for safety'
				};
			}

			try {
				// Execute in a read-only transaction for extra safety
				const client = await pool.connect();
				try {
					await client.query('BEGIN TRANSACTION READ ONLY');
					await client.query('SET search_path TO elt, public');

					const result = await client.query(query);

					await client.query('COMMIT');

					console.log(`[queryOntologiesTool] Returned ${result.rows.length} rows`);

					return {
						success: true,
						rowCount: result.rows.length,
						rows: result.rows,
						columns: result.fields.map((field) => field.name)
					};
				} catch (error) {
					await client.query('ROLLBACK');
					throw error;
				} finally {
					client.release();
				}
			} catch (error) {
				console.error('[queryOntologiesTool] Error:', error);
				return {
					success: false,
					error: error instanceof Error ? error.message : 'Unknown database error'
				};
			}
		}
	});
}