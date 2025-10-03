/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file is generated from: sources/ios/location/_stream.yaml (schema section)
 * To modify the schema, edit the source YAML file and regenerate.
 * 
 * Generated at: 2025-08-17T17:55:47.090Z
 */

import { index, integer, jsonb, pgTable, real, timestamp, uuid, varchar } from 'drizzle-orm/pg-core';
import { sources } from '../sources';

/**
 * GPS and location data from iOS Core Location
 */
export const streamIosLocation = pgTable('stream_ios_location', {
  // Common fields
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull(),
  
  // Stream-specific fields

  latitude: real('latitude').notNull(),  // Latitude coordinate,
  longitude: real('longitude').notNull(),  // Longitude coordinate,
  altitude: real('altitude'),  // Altitude in meters,
  horizontalAccuracy: real('horizontal_accuracy'),  // Horizontal accuracy in meters,
  verticalAccuracy: real('vertical_accuracy'),  // Vertical accuracy in meters,
  speed: real('speed'),  // Speed in meters per second,
  course: real('course'),  // Course/heading in degrees from true north,
  floor: integer('floor'),  // Floor level in building,
  activityType: varchar('activity_type', { length: 50 }),  // Type of activity (stationary, walking, running, automotive, etc.),
  address: varchar('address', { length: 500 }),  // Reverse geocoded address,
  placeName: varchar('place_name', { length: 200 }),  // Name of the place/venue,
  rawData: jsonb('raw_data'),  // Additional location metadata,
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  streamIosLocationTimestampIdx: index('stream_ios_location_timestamp_idx').on(table.timestamp),
  sourceIdIdx: index('stream_ios_location_source_id_idx').on(table.sourceId),
}));

// Type exports
export type StreamIosLocation = typeof streamIosLocation.$inferSelect;
export type NewStreamIosLocation = typeof streamIosLocation.$inferInsert;
