import { pgTable, varchar, timestamp, uuid, text, jsonb } from 'drizzle-orm/pg-core';

export const databaseUsers = pgTable('database_users', {
	id: uuid('id').primaryKey().defaultRandom(),
	name: varchar('name', { length: 255 }).notNull().unique(),
	permissionLevel: varchar('permission_level', { length: 50 }).notNull(), // 'readonly', 'readwrite', 'full'
	connectionStringEncrypted: text('connection_string_encrypted').notNull(),
	permissions: jsonb('permissions').$type<{
		grants: string[];
		description: string;
	}>().notNull(),
	createdAt: timestamp('created_at', { withTimezone: true }).defaultNow().notNull(),
	lastUsed: timestamp('last_used', { withTimezone: true }),
	revokedAt: timestamp('revoked_at', { withTimezone: true }),
});

export type DatabaseUser = typeof databaseUsers.$inferSelect;
export type NewDatabaseUser = typeof databaseUsers.$inferInsert;