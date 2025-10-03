/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file is generated from: sources/notion/pages/_stream.yaml (schema section)
 * To modify the schema, edit the source YAML file and regenerate.
 * 
 * Generated at: 2025-08-17T17:55:47.091Z
 */

import { boolean, index, jsonb, pgTable, text, time, timestamp, uuid, varchar } from 'drizzle-orm/pg-core';
import { sources } from '../sources';

/**
 * Pages and databases from Notion workspace
 */
export const streamNotionPages = pgTable('stream_notion_pages', {
  // Common fields
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull(),
  
  // Stream-specific fields

  pageId: varchar('page_id', { length: 100 }).notNull(),  // Notion page UUID,
  parentId: varchar('parent_id', { length: 100 }),  // Parent page or workspace ID,
  parentType: varchar('parent_type', { length: 20 }),  // Type of parent (page, database, workspace),
  title: varchar('title', { length: 500 }),  // Page title,
  objectType: varchar('object_type', { length: 20 }),  // Object type (page, database),
  archived: boolean('archived').default(false),  // Whether the page is archived,
  url: varchar('url', { length: 500 }),  // Public URL if shared,
  createdTime: timestamp('created_time', { withTimezone: true }),  // When the page was created in Notion,
  createdBy: varchar('created_by', { length: 100 }),  // User ID who created the page,
  lastEditedTime: timestamp('last_edited_time', { withTimezone: true }),  // When the page was last edited in Notion,
  lastEditedBy: varchar('last_edited_by', { length: 100 }),  // User ID who last edited the page,
  contentText: text('content_text'),  // Extracted plain text content,
  contentMarkdown: text('content_markdown'),  // Content converted to Markdown,
  properties: jsonb('properties'),  // Database properties and values,
  icon: jsonb('icon'),  // Page icon (emoji or image),
  cover: jsonb('cover'),  // Page cover image,
  parent: jsonb('parent'),  // Full parent relationship data,
  blocks: jsonb('blocks'),  // Page content blocks (if small),
  minioPath: varchar('minio_path', { length: 500 }),  // Path to full content in MinIO (if large),
  fullPage: jsonb('full_page'),  // Complete page object for unmapped fields,
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  streamNotionPagesTimestampIdx: index('stream_notion_pages_timestamp_idx').on(table.timestamp),
  streamNotionPagesLastEditedTimeIdx: index('stream_notion_pages_last_edited_time_idx').on(table.lastEditedTime),
  streamNotionPagesPageIdIdx: index('stream_notion_pages_page_id_idx').on(table.pageId),
  sourceIdIdx: index('stream_notion_pages_source_id_idx').on(table.sourceId),
}));

// Type exports
export type StreamNotionPages = typeof streamNotionPages.$inferSelect;
export type NewStreamNotionPages = typeof streamNotionPages.$inferInsert;
