import { resolve } from 'node:path';
import { config } from 'dotenv';
import { defineConfig } from 'drizzle-kit';

config({ path: resolve(__dirname, '../../.env') });

// Drizzle config for type-safe queries only
// Schema migrations are managed by SQLx in core/migrations/
export default defineConfig({
	schema: './src/lib/server/schema.ts',
	dialect: 'postgresql',
	dbCredentials: {
		url: process.env.DATABASE_URL || ''
	},
	schemaFilter: ['app']
	// Note: No 'out' field - we don't generate migrations here
	// All migrations are in core/migrations/ (SQLx)
});
