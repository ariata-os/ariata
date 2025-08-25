/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file is generated from: sources/mac/messages/_stream.yaml (schema section)
 * To modify the schema, edit the source YAML file and regenerate.
 * 
 * Generated at: 2025-08-17T22:17:34.164Z
 */

import { boolean, index, integer, jsonb, pgTable, text, time, timestamp, uuid, varchar } from 'drizzle-orm/pg-core';
import { sources } from '../sources';

/**
 * iMessage and SMS messages from macOS
 */
export const streamMacMessages = pgTable('stream_mac_messages', {
  // Common fields
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull(),
  
  // Stream-specific fields

  messageId: varchar('message_id', { length: 200 }).notNull(),  // Unique message identifier (GUID),
  chatId: varchar('chat_id', { length: 200 }).notNull(),  // Chat/conversation identifier,
  handleId: varchar('handle_id', { length: 200 }),  // Contact handle (phone/email),
  text: text('text'),  // Message content,
  service: varchar('service', { length: 50 }),  // Service type (iMessage, SMS),
  isFromMe: boolean('is_from_me').notNull().default(false),  // Whether message was sent by user,
  date: timestamp('date', { withTimezone: true }).notNull(),  // Message timestamp,
  dateRead: timestamp('date_read', { withTimezone: true }),  // When message was read,
  dateDelivered: timestamp('date_delivered', { withTimezone: true }),  // When message was delivered,
  isRead: boolean('is_read').default(false),  // Whether message has been read,
  isDelivered: boolean('is_delivered').default(false),  // Whether message was delivered,
  isSent: boolean('is_sent').default(false),  // Whether message was sent successfully,
  cacheHasAttachments: boolean('cache_has_attachments').default(false),  // Whether message has attachments,
  attachmentCount: integer('attachment_count'),  // Number of attachments,
  attachmentInfo: jsonb('attachment_info'),  // Attachment metadata (filenames, types, sizes),
  groupTitle: varchar('group_title', { length: 500 }),  // Group chat name if applicable,
  associatedMessageGuid: varchar('associated_message_guid', { length: 200 }),  // Related message ID (for replies/reactions),
  associatedMessageType: integer('associated_message_type'),  // Type of association (reply, reaction, etc),
  expressiveSendStyleId: varchar('expressive_send_style_id', { length: 100 }),  // Message effect style (invisible ink, etc),
  rawData: jsonb('raw_data'),  // Additional unmapped fields,
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  streamMacMessagesTimestampIdx: index('stream_mac_messages_timestamp_idx').on(table.timestamp),
  streamMacMessagesMessageIdIdx: index('stream_mac_messages_message_id_idx').on(table.messageId),
  streamMacMessagesChatIdDateIdx: index('stream_mac_messages_chat_id_date_idx').on(table.chatId, table.date),
  streamMacMessagesIsFromMeDateIdx: index('stream_mac_messages_is_from_me_date_idx').on(table.isFromMe, table.date),
  streamMacMessagesSourceIdMessageIdIdx: index('stream_mac_messages_source_id_message_id_idx').on(table.sourceId, table.messageId),
  sourceIdIdx: index('stream_mac_messages_source_id_idx').on(table.sourceId),
}));

// Type exports
export type StreamMacMessages = typeof streamMacMessages.$inferSelect;
export type NewStreamMacMessages = typeof streamMacMessages.$inferInsert;
