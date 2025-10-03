/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file is generated from: sources/ios/healthkit/_stream.yaml (schema section)
 * To modify the schema, edit the source YAML file and regenerate.
 * 
 * Generated at: 2025-08-17T17:55:47.090Z
 */

import { index, integer, jsonb, pgTable, real, timestamp, uuid, varchar } from 'drizzle-orm/pg-core';
import { sources } from '../sources';

/**
 * Health metrics from iOS HealthKit including heart rate, HRV, steps, and activity
 */
export const streamIosHealthkit = pgTable('stream_ios_healthkit', {
  // Common fields
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull(),
  
  // Stream-specific fields

  heartRate: real('heart_rate'),  // Heart rate in beats per minute,
  hrv: real('hrv'),  // Heart rate variability in milliseconds,
  activityType: varchar('activity_type', { length: 50 }),  // Type of activity (sleeping, walking, running, etc.),
  confidence: real('confidence'),  // Confidence level of the measurement,
  steps: integer('steps'),  // Number of steps,
  activeEnergy: real('active_energy'),  // Active energy burned in kcal,
  sleepStage: varchar('sleep_stage', { length: 20 }),  // Sleep stage (awake, light, deep, rem),
  workoutType: varchar('workout_type', { length: 50 }),  // Type of workout activity,
  workoutDuration: integer('workout_duration'),  // Workout duration in seconds,
  deviceName: varchar('device_name', { length: 100 }),  // Name of the device that recorded the data,
  rawData: jsonb('raw_data'),  // Additional fields not mapped to columns,
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  streamIosHealthkitTimestampIdx: index('stream_ios_healthkit_timestamp_idx').on(table.timestamp),
  sourceIdIdx: index('stream_ios_healthkit_source_id_idx').on(table.sourceId),
}));

// Type exports
export type StreamIosHealthkit = typeof streamIosHealthkit.$inferSelect;
export type NewStreamIosHealthkit = typeof streamIosHealthkit.$inferInsert;
