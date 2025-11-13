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
 * Tool call structure (future extension)
 */
export interface ToolCall {
	tool_name: string;
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
// Type Inference
// ============================================================================

export type ChatSession = typeof chatSessions.$inferSelect;
export type NewChatSession = typeof chatSessions.$inferInsert;
