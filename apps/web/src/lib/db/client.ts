import { drizzle } from 'drizzle-orm/postgres-js';
import postgres from 'postgres';
import * as schema from './schema';

// Get database URL from environment
// Docker Compose will override this for container networking
let databaseUrl = (process.env.DATABASE_URL || '')
  .replace('postgresql+asyncpg://', 'postgresql://');

// Handle both Docker and local development environments
const isDocker = process.env.NODE_ENV === 'production' || process.env.HOSTNAME || process.env.DOCKER_ENV;

if (!isDocker && databaseUrl) {
  // Local development: replace postgres hostname with localhost
  databaseUrl = databaseUrl.replace('@postgres:', '@localhost:');
}

if (!databaseUrl && process.env.NODE_ENV !== 'production') {
  console.warn('DATABASE_URL not set, using dummy URL for build');
}

const queryClient = postgres(databaseUrl);
export const db = drizzle(queryClient, { schema });
