import { pgTable, varchar, uuid, boolean, timestamp, json, text, index, uniqueIndex } from 'drizzle-orm/pg-core';
import { sourceConfigs } from './source_configs';

// Source status enum
export type SourceStatus = 'authenticated' | 'active' | 'paused' | 'needs_reauth' | 'error';

export const sources = pgTable('sources', {
  // Primary key
  id: uuid('id').primaryKey().defaultRandom().notNull(),


  sourceName: varchar('source_name')
    .references(() => sourceConfigs.name, { onDelete: 'restrict' })
    .notNull(),

  // Instance identification
  instanceName: varchar('instance_name').notNull(), // e.g., "John's iPhone", "Work Calendar"

  // Status
  status: varchar('status').$type<SourceStatus>().notNull().default('authenticated'),

  // Device-specific fields (for device sources)
  deviceId: varchar('device_id'), // Unique device identifier
  deviceToken: text('device_token'), // Authentication token for device
  deviceType: varchar('device_type'), // 'ios', 'mac', etc.
  deviceLastSeen: timestamp('device_last_seen', { withTimezone: true }),

  // OAuth-specific fields (for cloud sources)
  oauthAccessToken: text('oauth_access_token'),
  oauthRefreshToken: text('oauth_refresh_token'),
  oauthExpiresAt: timestamp('oauth_expires_at', { withTimezone: true }),
  scopes: json('scopes').$type<string[]>(),

  // Additional source metadata
  sourceMetadata: json('source_metadata').$type<Record<string, any>>().default({}),

  // Sync tracking
  lastSyncAt: timestamp('last_sync_at', { withTimezone: true }),
  lastSyncStatus: varchar('last_sync_status'), // 'success', 'failed', 'in_progress'
  lastSyncError: text('last_sync_error'),

  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  sourceNameIdx: index('sources_instances_source_name_idx').on(table.sourceName),
  deviceIdIdx: index('sources_instances_device_id_idx').on(table.deviceId),

  // Unique constraints
  // A user can have multiple instances of the same source type (e.g., multiple iOS devices)
  // But each device ID should be unique
  uniqueDeviceId: uniqueIndex('sources_instances_unique_device_id').on(table.deviceId),

  // For OAuth sources, ensure unique combination of user + source type + instance name
  uniqueUserSourceInstance: uniqueIndex('sources_instances_unique_user_source_instance').on(
    table.sourceName,
    table.instanceName
  ),
}));

// Type exports
export type Source = typeof sources.$inferSelect;
export type NewSource = typeof sources.$inferInsert;