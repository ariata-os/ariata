#!/usr/bin/env tsx
import * as dotenv from 'dotenv';
dotenv.config({ path: '../../.env' });

import { drizzle } from 'drizzle-orm/postgres-js';
import { migrate } from 'drizzle-orm/postgres-js/migrator';
import postgres from 'postgres';
import * as schema from '../src/lib/db/schema';

// Handle both Docker and local development environments
const rawUrl = process.env.DATABASE_URL || '';
let databaseUrl = rawUrl.replace('postgresql+asyncpg://', 'postgresql://');

// Simple environment detection: if NODE_ENV is not production or we detect Docker environment
const isDocker = process.env.NODE_ENV === 'production' || process.env.HOSTNAME || process.env.DOCKER_ENV;

if (!isDocker) {
  // Local development: replace postgres hostname with localhost
  databaseUrl = databaseUrl.replace('@postgres:', '@localhost:');
}

console.log('Environment detection:', { isDocker, NODE_ENV: process.env.NODE_ENV, HOSTNAME: process.env.HOSTNAME });
console.log('Final database URL:', databaseUrl);

async function runMigrations() {
  console.log('Running migrations...');
  
  // Create a connection specifically for migrations
  const migrationClient = postgres(databaseUrl, { max: 1 });
  const db = drizzle(migrationClient, { schema });
  
  try {
    await migrate(db, { migrationsFolder: './drizzle' });
    console.log('Migrations completed successfully');
  } catch (error) {
    console.error('Migration failed:', error);
    process.exit(1);
  } finally {
    await migrationClient.end();
  }
}

runMigrations();