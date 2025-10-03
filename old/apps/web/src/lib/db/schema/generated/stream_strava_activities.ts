/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file is generated from: sources/strava/activities/_stream.yaml (schema section)
 * To modify the schema, edit the source YAML file and regenerate.
 * 
 * Generated at: 2025-08-17T17:55:47.091Z
 */

import { bigint, index, integer, jsonb, pgTable, real, time, timestamp, uuid, varchar } from 'drizzle-orm/pg-core';
import { sources } from '../sources';

/**
 * Fitness activities and workouts from Strava
 */
export const streamStravaActivities = pgTable('stream_strava_activities', {
  // Common fields
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull(),
  
  // Stream-specific fields

  activityId: bigint('activity_id', { mode: 'number' }).notNull(),  // Strava activity ID,
  externalId: varchar('external_id', { length: 200 }),  // External ID from device/app,
  name: varchar('name', { length: 500 }),  // Activity name/title,
  type: varchar('type', { length: 50 }),  // Activity type (Run, Ride, Swim, etc.),
  sportType: varchar('sport_type', { length: 50 }),  // Specific sport type,
  workoutType: integer('workout_type'),  // Workout type code,
  distance: real('distance'),  // Distance in meters,
  movingTime: integer('moving_time'),  // Moving time in seconds,
  elapsedTime: integer('elapsed_time'),  // Total elapsed time in seconds,
  totalElevationGain: real('total_elevation_gain'),  // Total elevation gain in meters,
  elevHigh: real('elev_high'),  // Highest elevation in meters,
  elevLow: real('elev_low'),  // Lowest elevation in meters,
  averageSpeed: real('average_speed'),  // Average speed in meters per second,
  maxSpeed: real('max_speed'),  // Maximum speed in meters per second,
  averageHeartrate: real('average_heartrate'),  // Average heart rate in bpm,
  maxHeartrate: real('max_heartrate'),  // Maximum heart rate in bpm,
  averageCadence: real('average_cadence'),  // Average cadence,
  averageWatts: real('average_watts'),  // Average power in watts,
  kilojoules: real('kilojoules'),  // Total work in kilojoules,
  startDate: timestamp('start_date', { withTimezone: true }).notNull(),  // Activity start time (UTC),
  startDateLocal: timestamp('start_date_local', { withTimezone: true }),  // Activity start time (local),
  timezone: varchar('timezone', { length: 50 }),  // Timezone of the activity,
  achievementCount: integer('achievement_count'),  // Number of achievements,
  kudosCount: integer('kudos_count'),  // Number of kudos received,
  commentCount: integer('comment_count'),  // Number of comments,
  startLatlng: jsonb('start_latlng'),  // Starting coordinates [lat, lng],
  endLatlng: jsonb('end_latlng'),  // Ending coordinates [lat, lng],
  map: jsonb('map'),  // Map polyline and summary,
  splitsMetric: jsonb('splits_metric'),  // Kilometer splits,
  splitsStandard: jsonb('splits_standard'),  // Mile splits,
  segmentEfforts: jsonb('segment_efforts'),  // Segment efforts within activity,
  gear: jsonb('gear'),  // Equipment used,
  photos: jsonb('photos'),  // Activity photos metadata,
  stats: jsonb('stats'),  // Additional statistics,
  fullActivity: jsonb('full_activity'),  // Complete activity object for unmapped fields,
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  streamStravaActivitiesTimestampIdx: index('stream_strava_activities_timestamp_idx').on(table.timestamp),
  streamStravaActivitiesStartDateIdx: index('stream_strava_activities_start_date_idx').on(table.startDate),
  streamStravaActivitiesActivityIdIdx: index('stream_strava_activities_activity_id_idx').on(table.activityId),
  sourceIdIdx: index('stream_strava_activities_source_id_idx').on(table.sourceId),
}));

// Type exports
export type StreamStravaActivities = typeof streamStravaActivities.$inferSelect;
export type NewStreamStravaActivities = typeof streamStravaActivities.$inferInsert;
