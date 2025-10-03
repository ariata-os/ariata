/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file is generated from: sources/google/calendar/_stream.yaml (schema section)
 * To modify the schema, edit the source YAML file and regenerate.
 * 
 * Generated at: 2025-08-17T17:55:47.089Z
 */

import { boolean, index, jsonb, pgTable, text, time, timestamp, uuid, varchar } from 'drizzle-orm/pg-core';
import { sources } from '../sources';

/**
 * Calendar events and appointments from Google Calendar
 */
export const streamGoogleCalendar = pgTable('stream_google_calendar', {
  // Common fields
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull(),
  
  // Stream-specific fields

  eventId: varchar('event_id', { length: 200 }).notNull(),  // Google Calendar event ID,
  calendarId: varchar('calendar_id', { length: 200 }).notNull(),  // Calendar ID (email or calendar identifier),
  icalUid: varchar('ical_uid', { length: 200 }),  // iCalendar UID for the event,
  summary: varchar('summary', { length: 500 }),  // Event title/summary,
  description: text('description'),  // Event description,
  location: varchar('location', { length: 500 }),  // Event location,
  status: varchar('status', { length: 20 }),  // Event status (confirmed, tentative, cancelled),
  startTime: timestamp('start_time', { withTimezone: true }).notNull(),  // Event start time,
  endTime: timestamp('end_time', { withTimezone: true }).notNull(),  // Event end time,
  allDay: boolean('all_day').default(false),  // Whether this is an all-day event,
  timezone: varchar('timezone', { length: 50 }),  // Event timezone,
  htmlLink: varchar('html_link', { length: 500 }),  // Link to event in Google Calendar,
  createdTime: timestamp('created_time', { withTimezone: true }),  // When the event was created,
  updatedTime: timestamp('updated_time', { withTimezone: true }),  // When the event was last updated,
  eventType: varchar('event_type', { length: 50 }),  // Type of event (default, outOfOffice, focusTime, etc.),
  creator: jsonb('creator'),  // Event creator information,
  organizer: jsonb('organizer'),  // Event organizer information,
  attendees: jsonb('attendees'),  // List of attendees with response status,
  reminders: jsonb('reminders'),  // Reminder settings,
  recurrence: jsonb('recurrence'),  // Recurrence rules (RRULE),
  conferenceData: jsonb('conference_data'),  // Video/phone conference details,
  fullEvent: jsonb('full_event'),  // Complete event object for unmapped fields,
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  streamGoogleCalendarTimestampIdx: index('stream_google_calendar_timestamp_idx').on(table.timestamp),
  streamGoogleCalendarStartTimeIdx: index('stream_google_calendar_start_time_idx').on(table.startTime),
  streamGoogleCalendarEventIdIdx: index('stream_google_calendar_event_id_idx').on(table.eventId),
  sourceIdIdx: index('stream_google_calendar_source_id_idx').on(table.sourceId),
}));

// Type exports
export type StreamGoogleCalendar = typeof streamGoogleCalendar.$inferSelect;
export type NewStreamGoogleCalendar = typeof streamGoogleCalendar.$inferInsert;
