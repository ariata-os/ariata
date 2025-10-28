import { pgTable, text, timestamp, boolean, uuid } from 'drizzle-orm/pg-core';

/**
 * Application Database Schema
 *
 * Uses the 'app' schema within the ariata database.
 * It stores UI-specific state and user preferences.
 *
 * The 'elt' schema is managed by Rust/sqlx and contains:
 * - sources, streams, sync_logs, stream_* tables
 *
 * The 'app' schema is managed by SvelteKit/Drizzle and contains:
 * - User preferences, dashboards, saved queries, UI state
 *
 * Future: 'transform' schema for Python transformation layer
 */

// User preferences (theme, settings, etc.)
export const preferences = pgTable(
	'preferences',
	{
		key: text('key').primaryKey(),
		value: text('value').notNull(),
		updatedAt: timestamp('updated_at').notNull().defaultNow()
	},
	() => [],
	{ schema: 'app' }
);

// Saved dashboards and visualizations
export const dashboards = pgTable(
	'dashboards',
	{
		id: uuid('id').primaryKey().defaultRandom(),
		name: text('name').notNull(),
		description: text('description'),
		layout: text('layout').notNull(), // JSON string with widget positions
		isDefault: boolean('is_default').notNull().default(false),
		createdAt: timestamp('created_at').notNull().defaultNow(),
		updatedAt: timestamp('updated_at').notNull().defaultNow()
	},
	() => [],
	{ schema: 'app' }
);

// Saved queries for exploring data
export const savedQueries = pgTable(
	'saved_queries',
	{
		id: uuid('id').primaryKey().defaultRandom(),
		name: text('name').notNull(),
		description: text('description'),
		query: text('query').notNull(), // SQL query string
		sourceId: text('source_id'), // Optional: associated source from ELT schema
		createdAt: timestamp('created_at').notNull().defaultNow(),
		updatedAt: timestamp('updated_at').notNull().defaultNow()
	},
	() => [],
	{ schema: 'app' }
);

// Recently viewed sources (for quick access)
export const recentlyViewed = pgTable(
	'recently_viewed',
	{
		id: uuid('id').primaryKey().defaultRandom(),
		sourceId: text('source_id').notNull(), // References sources.id from elt schema
		sourceName: text('source_name').notNull(),
		sourceType: text('source_type').notNull(),
		viewedAt: timestamp('viewed_at').notNull().defaultNow()
	},
	() => [],
	{ schema: 'app' }
);
