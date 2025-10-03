/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file is generated from: sources/ios/mic/_stream.yaml (schema section)
 * To modify the schema, edit the source YAML file and regenerate.
 * 
 * Generated at: 2025-08-17T17:55:47.091Z
 */

import { index, integer, jsonb, pgTable, real, text, time, timestamp, uuid, varchar } from 'drizzle-orm/pg-core';
import { sources } from '../sources';

/**
 * Audio metadata and transcriptions from iOS microphone
 */
export const streamIosMic = pgTable('stream_ios_mic', {
  // Common fields
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull(),
  
  // Stream-specific fields

  recordingId: varchar('recording_id', { length: 100 }).notNull(),  // Unique identifier for the recording,
  timestampStart: timestamp('timestamp_start', { withTimezone: true }).notNull(),  // Start time of the recording,
  timestampEnd: timestamp('timestamp_end', { withTimezone: true }).notNull(),  // End time of the recording,
  duration: integer('duration').notNull(),  // Duration in milliseconds,
  overlapDuration: real('overlap_duration'),  // Overlap duration with previous recording in seconds,
  audioFormat: varchar('audio_format', { length: 10 }),  // Audio format (wav, mp3, etc.),
  sampleRate: integer('sample_rate'),  // Sample rate in Hz,
  audioLevel: real('audio_level'),  // Average audio level in dB,
  peakLevel: real('peak_level'),  // Peak audio level in dB,
  transcriptionText: text('transcription_text'),  // Transcribed text from audio,
  transcriptionConfidence: real('transcription_confidence'),  // Confidence score of transcription,
  language: varchar('language', { length: 10 }),  // Detected language code,
  minioPath: varchar('minio_path', { length: 500 }),  // Path to audio file in MinIO storage,
  fileSize: integer('file_size'),  // Size of audio file in bytes,
  rawData: jsonb('raw_data'),  // Additional metadata,
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  streamIosMicTimestampIdx: index('stream_ios_mic_timestamp_idx').on(table.timestamp),
  streamIosMicTimestampStartIdx: index('stream_ios_mic_timestamp_start_idx').on(table.timestampStart),
  sourceIdIdx: index('stream_ios_mic_source_id_idx').on(table.sourceId),
}));

// Type exports
export type StreamIosMic = typeof streamIosMic.$inferSelect;
export type NewStreamIosMic = typeof streamIosMic.$inferInsert;
