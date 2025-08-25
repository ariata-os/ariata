import { pgTable, uuid, varchar, text, timestamp, json, index, unique, check } from 'drizzle-orm/pg-core';
import { sql } from 'drizzle-orm';
import { sourceConfigs } from './source_configs';

// Streams represent raw data flows from sources before processing into signals
export const streamConfigs = pgTable('stream_configs', {
  // Primary key
  id: uuid('id').primaryKey().defaultRandom(),

  // Stream identification
  streamName: varchar('stream_name').notNull(), // e.g., 'apple_ios_core_location'
  sourceName: varchar('source_name')
    .notNull()
    .references(() => sourceConfigs.name, { onDelete: 'restrict' }),

  // Display information
  displayName: varchar('display_name').notNull(), // e.g., 'iOS Core Location'
  description: text('description'),

  // Stream configuration
  ingestionType: varchar('ingestion_type').notNull(), // 'push' or 'pull'
  status: varchar('status').notNull().default('active'),

  // Sync configuration
  cronSchedule: varchar('cron_schedule'), // Cron schedule for pull-based streams

  // Configuration
  settings: json('settings').$type<Record<string, any>>(),

  // Timestamps
  lastIngestionAt: timestamp('last_ingestion_at', { withTimezone: true }),
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  sourceNameIdx: index('streams_source_name_idx').on(table.sourceName),
  streamNameIdx: index('streams_stream_name_idx').on(table.streamName),

  // Check constraints
  ingestionTypeCheck: check(
    'streams_ingestion_type_check',
    sql`${table.ingestionType} IN ('push', 'pull')`
  ),
  statusCheck: check(
    'streams_status_check',
    sql`${table.status} IN ('active', 'paused', 'inactive')`
  ),
}));

// Type exports
export type StreamConfig = typeof streamConfigs.$inferSelect;
export type NewStreamConfig = typeof streamConfigs.$inferInsert;