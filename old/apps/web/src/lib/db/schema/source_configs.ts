import { pgTable, varchar, real, json, timestamp, check, text, boolean, index, integer } from 'drizzle-orm/pg-core';
import { sql } from 'drizzle-orm';

export const sourceConfigs = pgTable('source_configs', {
  // Primary key
  name: varchar('name').primaryKey().notNull(),

  // Core fields
  company: varchar('company').notNull(),
  platform: varchar('platform').notNull().default('cloud'), // 'cloud' or 'device'

  // Device-specific fields (for platform='device')
  deviceType: varchar('device_type'), // 'ios', 'android', 'macos', etc.

  // Fidelity configuration
  defaultFidelityScore: real('default_fidelity_score')
    .notNull()
    .default(1.0),

  // Auth type
  authType: varchar('auth_type').default('oauth2'),

  // Display information
  displayName: varchar('display_name'),
  description: text('description'),
  icon: varchar('icon'), // Remix icon identifier (e.g., 'ri:google-fill')
  video: varchar('video'), // Video filename (e.g., 'google2.webm')

  // Configuration
  oauthConfig: json('oauth_config').$type<{
    authProxy?: string;
    requiredScopes?: string[];
  }>(),
  syncConfig: json('sync_config').$type<Record<string, any>>(),
  
  // Sync settings
  defaultSyncSchedule: varchar('default_sync_schedule'), // Default cron schedule for this source
  minSyncFrequency: integer('min_sync_frequency'), // Minimum sync frequency in seconds
  maxSyncFrequency: integer('max_sync_frequency'), // Maximum sync frequency in seconds

  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  platformIdx: index('sources_platform_idx').on(table.platform),

  // Check constraints
  fidelityScoreCheck: check(
    'fidelity_score_check',
    sql`${table.defaultFidelityScore} >= 0 AND ${table.defaultFidelityScore} <= 1`
  ),
}));

// Type exports
export type SourceConfig = typeof sourceConfigs.$inferSelect;
export type NewSourceConfig = typeof sourceConfigs.$inferInsert;