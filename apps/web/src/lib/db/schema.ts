import { pgSchema, text, timestamp, boolean, uuid } from 'drizzle-orm/pg-core';

/**
 * Application Database Schema
 *
 * Uses the 'app' schema within the ariata database.
 * It stores UI-specific state and user preferences.
 *
 * The 'data' schema is managed by Rust/sqlx and contains:
 * - sources, streams, sync_logs, stream_* tables
 * - All ontology tables (health, social, location, activity, knowledge)
 * - Axiology and actions tables
 * - User profile
 *
 * The 'app' schema is managed by Rust/sqlx and contains:
 * - Assistant profile, chat sessions
 * - Models, agents, tools
 * - API usage tracking
 */

// Define the 'app' schema
export const appSchema = pgSchema('app');
