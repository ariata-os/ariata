import 'dotenv/config';
import { drizzle } from 'drizzle-orm/node-postgres';

/**
 * Application Database Client
 *
 * This connects to the ariata database (app schema).
 * Used for UI-specific state: chat sessions, models, agents, tools, API usage.
 *
 * The data schema contains pipeline data and is managed by Rust.
 */

export const db = drizzle(process.env.DATABASE_URL!);
