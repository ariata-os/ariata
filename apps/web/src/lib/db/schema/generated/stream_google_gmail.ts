import { pgTable, text, timestamp, uuid, jsonb, boolean, integer, index, varchar } from 'drizzle-orm/pg-core';
import { sql } from 'drizzle-orm';

export const stream_google_gmail = pgTable('stream_google_gmail', {
  // Base columns
  id: uuid('id').primaryKey().defaultRandom(),
  stream_id: uuid('stream_id').notNull(),
  source_id: uuid('source_id').notNull(),
  user_id: uuid('user_id').notNull(),
  timestamp: timestamp('timestamp', { mode: 'date' }).notNull(),
  created_at: timestamp('created_at', { mode: 'date' }).defaultNow().notNull(),
  updated_at: timestamp('updated_at', { mode: 'date' }).defaultNow().notNull(),
  
  // Gmail-specific columns
  message_id: varchar('message_id', { length: 200 }).notNull(),
  thread_id: varchar('thread_id', { length: 200 }).notNull(),
  history_id: varchar('history_id', { length: 50 }),
  subject: varchar('subject', { length: 500 }),
  snippet: varchar('snippet', { length: 500 }),
  body_text: text('body_text'),
  body_html: text('body_html'),
  from_email: varchar('from_email', { length: 255 }),
  from_name: varchar('from_name', { length: 255 }),
  to_emails: jsonb('to_emails'),
  cc_emails: jsonb('cc_emails'),
  bcc_emails: jsonb('bcc_emails'),
  reply_to_emails: jsonb('reply_to_emails'),
  labels: jsonb('labels'),
  categories: jsonb('categories'),
  is_read: boolean('is_read').default(false),
  is_starred: boolean('is_starred').default(false),
  is_important: boolean('is_important').default(false),
  is_spam: boolean('is_spam').default(false),
  is_trash: boolean('is_trash').default(false),
  has_attachments: boolean('has_attachments').default(false),
  attachment_count: integer('attachment_count').default(0),
  attachments: jsonb('attachments'),
  size_bytes: integer('size_bytes'),
  received_date: timestamp('received_date', { mode: 'date' }).notNull(),
  sent_date: timestamp('sent_date', { mode: 'date' }),
  headers: jsonb('headers'),
  full_message: jsonb('full_message')
}, (table) => ({
  timestampIdx: index('stream_google_gmail_timestamp_idx').on(table.timestamp),
  receivedDateIdx: index('stream_google_gmail_received_date_idx').on(table.received_date),
  messageIdIdx: index('stream_google_gmail_message_id_idx').on(table.message_id),
  threadIdIdx: index('stream_google_gmail_thread_id_idx').on(table.thread_id),
  fromEmailIdx: index('stream_google_gmail_from_email_idx').on(table.from_email),
  statusIdx: index('stream_google_gmail_status_idx').on(table.is_read, table.is_starred)
}));
