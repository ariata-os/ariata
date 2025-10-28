import 'dotenv/config';
import { drizzle } from 'drizzle-orm/node-postgres';

/**
 * Application Database Client
 *
 * This connects to the ariata_app database (separate from ariata_elt).
 * Used for UI-specific state: preferences, dashboards, saved queries.
 *
 * The ELT database (ariata_elt) is managed by Rust and accessed via API.
 */

export const db = drizzle(process.env.DATABASE_URL!);
