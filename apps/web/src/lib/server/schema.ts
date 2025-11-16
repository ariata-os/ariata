/**
 * Drizzle ORM Schema for App Database
 *
 * IMPORTANT: This file is a TYPE DEFINITION LAYER ONLY.
 *
 * - Schema migrations: Managed by SQLx in core/migrations/ (single source of truth)
 * - This file: TypeScript types for Drizzle ORM queries (must be manually kept in sync)
 * - After changing migrations, update this file to match the SQL schema
 *
 * Defines the app schema tables (operational data for fast UI queries)
 * Separate from ELT schema (analytical data warehouse)
 */

import { boolean, integer, jsonb, pgSchema, text, timestamp, uuid } from 'drizzle-orm/pg-core';
import { sql } from 'drizzle-orm';

// App schema namespace
export const appSchema = pgSchema('app');

// ELT schema namespace (for sources table)
export const eltSchema = pgSchema('elt');

// ============================================================================
// Type Definitions
// ============================================================================

/**
 * Chat message structure stored in JSONB array
 */
export interface ChatMessage {
	role: 'user' | 'assistant' | 'system';
	content: string;
	timestamp: string; // ISO 8601 timestamp
	model?: string | null; // Claude model used (null for user messages)
	provider?: string; // AI provider (e.g., "anthropic")
	agentId?: string; // Agent that handled this message (e.g., "analytics", "research", "general")
	tool_calls?: ToolCall[]; // Record tool invocations
	intent?: IntentMetadata; // Intent classification metadata (assistant messages only)
}

/**
 * Intent classification metadata for analytics and debugging
 */
export interface IntentMetadata {
	type: 'data_query' | 'visualization' | 'analysis' | 'general_chat';
	confidence: number;
	reasoning: string;
	entities?: string[];
	timeRange?: {
		start?: string;
		end?: string;
	};
}

/**
 * Tool call structure
 * Stores tool invocations with results for persistence
 */
export interface ToolCall {
	tool_name: string;
	tool_call_id?: string; // AI SDK toolCallId for matching results
	arguments: Record<string, unknown>;
	result?: unknown;
	timestamp: string;
}

// ============================================================================
// Tables
// ============================================================================

/**
 * Chat sessions table
 *
 * Stores chat conversations with denormalized JSONB messages array.
 * Optimized for:
 * - Fast session list queries (ORDER BY updated_at DESC)
 * - Single query to load entire conversation
 * - Atomic message appends (array_append)
 */
export const chatSessions = appSchema.table('chat_sessions', {
	id: uuid('id').primaryKey().defaultRandom(),
	title: text('title').notNull(),
	messages: jsonb('messages').$type<ChatMessage[]>().notNull().default(sql`'[]'::jsonb`),
	createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
	updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
	messageCount: integer('message_count').notNull().default(0)
});

// ============================================================================
// Type Inference
// ============================================================================

// ============================================================================
// Preferences Table
// ============================================================================

/**
 * User preferences table
 * Key-value store for user settings (name, system prompt, etc.)
 */
export const preferences = appSchema.table('preferences', {
	key: text('key').primaryKey(),
	value: text('value').notNull(),
	updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
});

// ============================================================================
// Dashboards Table
// ============================================================================

/**
 * Saved dashboards and visualizations
 */
export const dashboards = appSchema.table('dashboards', {
	id: uuid('id').primaryKey().defaultRandom(),
	name: text('name').notNull(),
	description: text('description'),
	layout: text('layout').notNull(), // JSON string with widget positions
	isDefault: boolean('is_default').notNull().default(false),
	createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
	updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
});

// ============================================================================
// Saved Queries Table
// ============================================================================

/**
 * Saved queries for exploring data
 */
export const savedQueries = appSchema.table('saved_queries', {
	id: uuid('id').primaryKey().defaultRandom(),
	name: text('name').notNull(),
	description: text('description'),
	query: text('query').notNull(), // SQL query string
	sourceId: text('source_id'), // Optional: associated source from ELT schema
	createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
	updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
});

// ============================================================================
// Recently Viewed Table
// ============================================================================

/**
 * Recently viewed sources (for quick access)
 */
export const recentlyViewed = appSchema.table('recently_viewed', {
	id: uuid('id').primaryKey().defaultRandom(),
	sourceId: text('source_id').notNull(), // References sources.id from elt schema
	sourceName: text('source_name').notNull(),
	provider: text('provider').notNull(),
	viewedAt: timestamp('viewed_at', { withTimezone: true }).notNull().defaultNow()
});

// ============================================================================
// Sources Table (from ELT schema)
// ============================================================================

/**
 * Data sources - supports both OAuth (Google, Notion) and Device (Mac, iOS)
 * Device pairing uses: device_id, device_token, pairing_status, device_info
 */
export const sources = eltSchema.table('sources', {
	id: uuid('id').primaryKey().defaultRandom(),
	provider: text('provider').notNull(),
	name: text('name').notNull().unique(),

	// OAuth credentials (null for device sources)
	accessToken: text('access_token'),
	refreshToken: text('refresh_token'),
	tokenExpiresAt: timestamp('token_expires_at', { withTimezone: true }),

	// Device authentication (used for Mac/iOS pairing)
	authType: text('auth_type').notNull().default('oauth2'),
	deviceId: text('device_id'),
	deviceInfo: jsonb('device_info').$type<{
		deviceName?: string;
		deviceType?: 'mac' | 'ios';
		osVersion?: string;
		model?: string;
	}>(),
	deviceToken: text('device_token'),
	pairingCode: text('pairing_code'),
	pairingStatus: text('pairing_status'), // 'pending' | 'active' | 'revoked'
	codeExpiresAt: timestamp('code_expires_at', { withTimezone: true }),
	lastSeenAt: timestamp('last_seen_at', { withTimezone: true }),

	// Status tracking
	isActive: boolean('is_active').default(true),
	isInternal: boolean('is_internal').default(false),
	errorMessage: text('error_message'),
	errorAt: timestamp('error_at', { withTimezone: true }),

	// Timestamps
	createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
	updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
});

// ============================================================================
// Assistant Profile Table (from ELT schema)
// ============================================================================

/**
 * Assistant profile - AI assistant preferences (singleton table)
 * Contains user's AI assistant configuration like name, default agent, default model
 */
export const assistantProfile = eltSchema.table('assistant_profile', {
	id: uuid('id').primaryKey(),
	assistantName: text('assistant_name'),
	defaultAgentId: text('default_agent_id'),
	defaultModelId: text('default_model_id'),
	createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
	updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow()
});

// ============================================================================
// Type Inference
// ============================================================================

export type ChatSession = typeof chatSessions.$inferSelect;
export type NewChatSession = typeof chatSessions.$inferInsert;
export type Source = typeof sources.$inferSelect;
export type NewSource = typeof sources.$inferInsert;
export type AssistantProfile = typeof assistantProfile.$inferSelect;
export type NewAssistantProfile = typeof assistantProfile.$inferInsert;
