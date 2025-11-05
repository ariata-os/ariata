import pg from 'pg';
import { drizzle, type NodePgDatabase } from 'drizzle-orm/node-postgres';
import { env } from '$env/dynamic/private';
import * as schema from './schema';

const { Pool } = pg;

let pool: pg.Pool | null = null;
let db: NodePgDatabase<typeof schema> | null = null;

export function getPool(): pg.Pool {
	if (!pool) {
		const databaseUrl = env.DATABASE_URL;
		if (!databaseUrl) {
			throw new Error('DATABASE_URL environment variable is not set');
		}
		pool = new Pool({ connectionString: databaseUrl });
	}
	return pool;
}

export function getDb(): NodePgDatabase<typeof schema> {
	if (!db) {
		db = drizzle(getPool(), { schema });
	}
	return db;
}