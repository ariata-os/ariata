import { pgTable, uuid, boolean, varchar, integer, json, timestamp, index, unique } from 'drizzle-orm/pg-core';
import { sources } from './sources';
import { streamConfigs } from './stream_configs';

// Streams table for source-stream instance configurations
// This tracks which streams are enabled for each source instance and their custom settings
export const streams = pgTable('streams', {
  // Primary key
  id: uuid('id').primaryKey().defaultRandom(),

  // Foreign keys
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  streamConfigId: uuid('stream_config_id')
    .notNull()
    .references(() => streamConfigs.id, { onDelete: 'restrict' }),

  // Stream configuration
  enabled: boolean('enabled').notNull().default(true),
  
  // Sync configuration
  syncSchedule: varchar('sync_schedule'), // Custom cron schedule (overrides stream_config default)
  initialSyncType: varchar('initial_sync_type').default('limited'), // 'limited' or 'full'
  initialSyncDays: integer('initial_sync_days').default(90), // Days to sync for limited initial sync (past)
  initialSyncDaysFuture: integer('initial_sync_days_future').default(30), // Days to sync for limited initial sync (future)
  
  // Custom settings for this stream instance
  settings: json('settings').$type<Record<string, any>>().default({}),
  
  // Sync tracking
  lastSyncAt: timestamp('last_sync_at', { withTimezone: true }),
  lastSyncStatus: varchar('last_sync_status'), // 'success', 'failed', 'in_progress'
  lastSyncError: varchar('last_sync_error'),
  syncCursor: varchar('sync_cursor'), // Stores sync tokens/cursors for incremental sync
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  sourceIdIdx: index('streams_source_id_idx').on(table.sourceId),
  streamConfigIdIdx: index('streams_stream_config_id_idx').on(table.streamConfigId),
  
  // Unique constraint - each source can only have one instance of each stream type
  uniqueSourceStream: unique('streams_unique_source_stream').on(table.sourceId, table.streamConfigId),
}));

// Type exports
export type Stream = typeof streams.$inferSelect;
export type NewStream = typeof streams.$inferInsert;