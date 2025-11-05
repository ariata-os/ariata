/**
 * Drizzle ORM Schema for App Database
 *
 * Defines the app schema tables (operational data for fast UI queries)
 * Separate from ELT schema (analytical data warehouse)
 */

import { pgTable, uuid, text, jsonb, timestamp, integer, pgSchema } from 'drizzle-orm/pg-core';
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
	tool_calls?: ToolCall[]; // Future: record tool invocations
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

export type ChatSession = typeof chatSessions.$inferSelect;
export type NewChatSession = typeof chatSessions.$inferInsert;
