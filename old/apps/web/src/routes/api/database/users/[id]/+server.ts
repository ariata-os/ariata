import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db/client';
import { databaseUsers } from '$lib/db/schema';
import { eq } from 'drizzle-orm';
import pkg from 'pg';
const { Client } = pkg;

// DELETE /api/database/users/:id - Revoke a database user
export const DELETE: RequestHandler = async ({ params }) => {
	try {
		const userId = params.id;

		// Get the user info
		const [user] = await db
			.select()
			.from(databaseUsers)
			.where(eq(databaseUsers.id, userId))
			.limit(1);

		if (!user) {
			return error(404, 'User not found');
		}

		// Connect to PostgreSQL as superuser
		const adminClient = new Client({
			connectionString: process.env.DATABASE_URL?.replace('+asyncpg', '').replace('postgresql://', 'postgres://')
		});

		await adminClient.connect();

		try {
			// Terminate any active connections for this user
			await adminClient.query(`
				SELECT pg_terminate_backend(pg_stat_activity.pid)
				FROM pg_stat_activity
				WHERE pg_stat_activity.usename = '${user.name}'
			`);

			// Revoke all privileges from the user
			await adminClient.query(`REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA public FROM ${user.name}`);
			await adminClient.query(`REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public FROM ${user.name}`);
			await adminClient.query(`REVOKE ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public FROM ${user.name}`);
			await adminClient.query(`REVOKE ALL PRIVILEGES ON SCHEMA public FROM ${user.name}`);
			await adminClient.query(`REVOKE ALL PRIVILEGES ON DATABASE ariata FROM ${user.name}`);
			
			// Revoke default privileges
			await adminClient.query(`ALTER DEFAULT PRIVILEGES IN SCHEMA public REVOKE ALL ON TABLES FROM ${user.name}`);
			await adminClient.query(`ALTER DEFAULT PRIVILEGES IN SCHEMA public REVOKE ALL ON SEQUENCES FROM ${user.name}`);
			await adminClient.query(`ALTER DEFAULT PRIVILEGES IN SCHEMA public REVOKE ALL ON FUNCTIONS FROM ${user.name}`);

			// Drop the PostgreSQL user
			await adminClient.query(`DROP USER IF EXISTS ${user.name}`);

			// Mark as revoked in our database
			await db
				.update(databaseUsers)
				.set({ revokedAt: new Date() })
				.where(eq(databaseUsers.id, userId));

			return json({ success: true });

		} finally {
			await adminClient.end();
		}

	} catch (err) {
		console.error('Failed to revoke database user:', err);
		return error(500, 'Failed to revoke database user');
	}
};