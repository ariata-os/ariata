import { pgTable, uuid, varchar, integer, timestamp, text, jsonb, pgEnum, index } from 'drizzle-orm/pg-core';
import { streams } from './streams';
import { users } from './users';

// Create enum for activity types
export const activityTypeEnum = pgEnum('activity_type', [
  'ingestion',
  'signal_creation',
  'transition_detection',
  'token_refresh',
  'scheduled_check',
  'cleanup'
]);

// Create enum for activity status
export const activityStatusEnum = pgEnum('activity_status', [
  'pending',
  'running',
  'completed',
  'failed',
  'cancelled'
]);

export const pipelineActivities = pgTable('pipeline_activities', {
  // Primary key
  id: uuid('id').primaryKey().defaultRandom(),

  // Activity identification
  activityType: activityTypeEnum('activity_type').notNull(),
  activityName: varchar('activity_name').notNull(), // e.g., 'apple_ios_core_location_ingestion'

  // Related entities (nullable based on activity type)
  streamId: uuid('stream_id')
    .references(() => streams.id, { onDelete: 'set null' }),
  sourceName: varchar('source_name').notNull(),

  // Status and timing
  status: activityStatusEnum('status').notNull().default('pending'),
  startedAt: timestamp('started_at', { withTimezone: true }),
  completedAt: timestamp('completed_at', { withTimezone: true }),

  // Metrics
  recordsProcessed: integer('records_processed'),
  dataSizeBytes: integer('data_size_bytes'),

  // Results
  outputPath: varchar('output_path'), // MinIO path or other storage location
  activityMetadata: jsonb('activity_metadata').$type<Record<string, any>>(), // Flexible field for activity-specific data
  errorMessage: text('error_message'),

  // Base timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes for common queries
  activityTypeIdx: index('pipeline_activities_activity_type_idx').on(table.activityType),
  statusIdx: index('pipeline_activities_status_idx').on(table.status),
  streamIdIdx: index('pipeline_activities_stream_id_idx').on(table.streamId),
  createdAtIdx: index('pipeline_activities_created_at_idx').on(table.createdAt),

}));

// Type exports
export type PipelineActivity = typeof pipelineActivities.$inferSelect;
export type NewPipelineActivity = typeof pipelineActivities.$inferInsert;
export type ActivityType = 'ingestion' | 'signal_creation' | 'transition_detection' | 'token_refresh' | 'scheduled_check' | 'cleanup';
export type ActivityStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';