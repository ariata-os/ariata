/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file is generated from: sources/mac/apps/_stream.yaml (schema section)
 * To modify the schema, edit the source YAML file and regenerate.
 * 
 * Generated at: 2025-08-17T17:55:47.091Z
 */

import { index, pgTable, timestamp, uuid, varchar } from 'drizzle-orm/pg-core';
import { sources } from '../sources';

/**
 * Application focus events from macOS
 */
export const streamMacApps = pgTable('stream_mac_apps', {
  // Common fields
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull(),
  
  // Stream-specific fields

  appName: varchar('app_name', { length: 200 }).notNull(),  // Application name,
  bundleId: varchar('bundle_id', { length: 200 }),  // macOS bundle identifier,
  eventType: varchar('event_type', { length: 50 }).notNull(),  // Event type (focus_gained, focus_lost, launch, quit),
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  streamMacAppsTimestampIdx: index('stream_mac_apps_timestamp_idx').on(table.timestamp),
  streamMacAppsAppNameTimestampIdx: index('stream_mac_apps_app_name_timestamp_idx').on(table.appName, table.timestamp),
  streamMacAppsEventTypeTimestampIdx: index('stream_mac_apps_event_type_timestamp_idx').on(table.eventType, table.timestamp),
  sourceIdIdx: index('stream_mac_apps_source_id_idx').on(table.sourceId),
}));

// Type exports
export type StreamMacApps = typeof streamMacApps.$inferSelect;
export type NewStreamMacApps = typeof streamMacApps.$inferInsert;
