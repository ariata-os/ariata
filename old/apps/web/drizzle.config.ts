import { defineConfig } from 'drizzle-kit';
import * as dotenv from 'dotenv';

dotenv.config();

if (!process.env.DATABASE_URL) {
  throw new Error('DATABASE_URL environment variable is required');
}

export default defineConfig({
  dialect: 'postgresql',
  schema: './src/lib/db/schema/*.ts',
  out: './drizzle',
  dbCredentials: {
    url: process.env.DATABASE_URL,
  },
  verbose: true,
  strict: true,
  tablesFilter: ["!spatial_ref_sys", "!geography_columns", "!geometry_columns", "!topology", "!layer", "!raster_columns", "!raster_overviews"],
  // Uncomment to enable automatic Python generation on every migration
  // Not set up yet, but a likely addition in the future
  // plugins: [
  //   pythonGenerator({
  //     output: '../../packages/db/models/generated',
  //     watch: true,
  //   })
  // ]
});