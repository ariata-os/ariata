import { pgTable, uuid, varchar, timestamp, index } from 'drizzle-orm/pg-core';

export const users = pgTable('users', {
  // Primary key
  id: uuid('id').primaryKey().defaultRandom(),

  // User information
  firstName: varchar('first_name').notNull(),
  lastName: varchar('last_name').notNull(),
  email: varchar('email').notNull().unique(),

  // User preferences
  timezone: varchar('timezone', { length: 50 }).notNull().default('America/Chicago'),

  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}, (table) => ({
  // Indexes
  emailIdx: index('users_email_idx').on(table.email),
}));

// Type exports
export type User = typeof users.$inferSelect;
export type NewUser = typeof users.$inferInsert;