import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db/client';
import { databaseUsers } from '$lib/db/schema';
import { eq, isNull } from 'drizzle-orm';
import pkg from 'pg';
const { Client } = pkg;
import crypto from 'crypto';

// Permission level configurations
const PERMISSION_LEVELS = {
	readonly: {
		grants: ['SELECT'],
		description: 'Read-only access to all tables'
	},
	readwrite: {
		grants: ['SELECT', 'INSERT', 'UPDATE'],
		description: 'Read and write access (no DELETE)'
	},
	full: {
		grants: ['SELECT', 'INSERT', 'UPDATE', 'DELETE'],
		description: 'Full access including DELETE operations'
	}
};

// Simple encryption for connection strings (in production, use proper KMS)
function encrypt(text: string): string {
	const algorithm = 'aes-256-cbc';
	const key = Buffer.from(process.env.ENCRYPTION_KEY || 'default-32-char-encryption-key!!', 'utf8').slice(0, 32);
	const iv = crypto.randomBytes(16);
	const cipher = crypto.createCipheriv(algorithm, key, iv);
	let encrypted = cipher.update(text, 'utf8', 'hex');
	encrypted += cipher.final('hex');
	return iv.toString('hex') + ':' + encrypted;
}

function decrypt(text: string): string {
	const algorithm = 'aes-256-cbc';
	const key = Buffer.from(process.env.ENCRYPTION_KEY || 'default-32-char-encryption-key!!', 'utf8').slice(0, 32);
	const parts = text.split(':');
	const iv = Buffer.from(parts[0], 'hex');
	const encryptedText = parts[1];
	const decipher = crypto.createDecipheriv(algorithm, key, iv);
	let decrypted = decipher.update(encryptedText, 'hex', 'utf8');
	decrypted += decipher.final('utf8');
	return decrypted;
}

// GET /api/database/users - List all database users
export const GET: RequestHandler = async ({ url }) => {
	try {
		const includeRevoked = url.searchParams.get('includeRevoked') === 'true';
		
		const query = db
			.select({
				id: databaseUsers.id,
				name: databaseUsers.name,
				permissionLevel: databaseUsers.permissionLevel,
				createdAt: databaseUsers.createdAt,
				lastUsed: databaseUsers.lastUsed,
				revokedAt: databaseUsers.revokedAt
			})
			.from(databaseUsers);
		
		// Only filter out revoked users if not explicitly including them
		if (!includeRevoked) {
			query.where(isNull(databaseUsers.revokedAt));
		}

		const users = await query;
		return json(users);
	} catch (err) {
		console.error('Failed to fetch database users:', err);
		return error(500, 'Failed to fetch database users');
	}
};

// POST /api/database/users - Create a new database user
export const POST: RequestHandler = async ({ request }) => {
	try {
		const { permissionLevel } = await request.json();

		if (!PERMISSION_LEVELS[permissionLevel as keyof typeof PERMISSION_LEVELS]) {
			return error(400, 'Invalid permission level');
		}

		// Generate a unique username and secure password
		const timestamp = Date.now();
		const randomStr = crypto.randomBytes(4).toString('hex');
		const username = `jaces_${permissionLevel}_${timestamp}_${randomStr}`;
		const password = crypto.randomBytes(16).toString('base64').replace(/[^a-zA-Z0-9]/g, '');

		// Connect to PostgreSQL as superuser to create the new user
		const adminClient = new Client({
			connectionString: process.env.DATABASE_URL?.replace('+asyncpg', '').replace('postgresql://', 'postgres://')
		});

		await adminClient.connect();

		try {
			// Create the PostgreSQL user
			await adminClient.query(`CREATE USER ${username} WITH PASSWORD '${password}'`);

			// Grant connection and schema permissions
			await adminClient.query(`GRANT CONNECT ON DATABASE ${process.env.DB_NAME || 'jaces'} TO ${username}`);
			await adminClient.query(`GRANT USAGE ON SCHEMA public TO ${username}`);

			// Grant table permissions based on permission level
			const grants = PERMISSION_LEVELS[permissionLevel as keyof typeof PERMISSION_LEVELS].grants.join(', ');
			
			// Grant permissions on all existing tables
			await adminClient.query(`GRANT ${grants} ON ALL TABLES IN SCHEMA public TO ${username}`);
			
			// Grant permissions on sequences (for INSERT operations)
			if (permissionLevel !== 'readonly') {
				await adminClient.query(`GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO ${username}`);
			}

			// Set default privileges for future tables
			await adminClient.query(`ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ${grants} ON TABLES TO ${username}`);
			
			if (permissionLevel !== 'readonly') {
				await adminClient.query(`ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE ON SEQUENCES TO ${username}`);
			}

			// Build connection string
			const dbHost = process.env.DB_HOST || 'localhost';
			const dbPort = process.env.DB_PORT || '5432';
			const dbName = process.env.DB_NAME || 'jaces';
			const connectionString = `postgresql://${username}:${password}@${dbHost}:${dbPort}/${dbName}`;

			// Store user info in our database
			const [newUser] = await db
				.insert(databaseUsers)
				.values({
					name: username,
					permissionLevel,
					connectionStringEncrypted: encrypt(connectionString),
					permissions: PERMISSION_LEVELS[permissionLevel as keyof typeof PERMISSION_LEVELS]
				})
				.returning({ id: databaseUsers.id });

			return json({
				success: true,
				connectionString,
				username,
				id: newUser.id
			});

		} catch (dbError) {
			// If user creation failed, clean up
			try {
				await adminClient.query(`DROP USER IF EXISTS ${username}`);
			} catch (cleanupError) {
				console.error('Failed to clean up user:', cleanupError);
			}
			throw dbError;
		} finally {
			await adminClient.end();
		}

	} catch (err) {
		console.error('Failed to create database user:', err);
		return error(500, `Failed to create database user: ${err instanceof Error ? err.message : 'Unknown error'}`);
	}
};

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