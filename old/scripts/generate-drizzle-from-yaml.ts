#!/usr/bin/env tsx
/**
 * Generate Drizzle TypeScript schemas from YAML stream definitions
 * 
 * This script reads _stream.yaml files from the sources directory and generates
 * corresponding Drizzle ORM schemas in TypeScript from the embedded schema section.
 * 
 * Usage: pnpm tsx scripts/generate-drizzle-from-yaml.ts
 */

import * as fs from 'fs';
import * as path from 'path';
import * as yaml from 'js-yaml';
import { glob } from 'glob';

interface YamlColumn {
  name: string;
  type: string;
  max_length?: number;
  nullable?: boolean;
  description?: string;
  default?: any;
  min?: number;
  max?: number;
}

interface YamlIndex {
  columns: string[];
  type?: string;
  description?: string;
}

interface YamlSchema {
  table_name: string;
  description: string;
  columns: YamlColumn[];
  indexes?: YamlIndex[];
  storage?: {
    strategy?: string;
    minio_fields?: string[];
    note?: string;
  };
}

interface StreamConfig {
  name: string;
  source: string;
  display_name: string;
  description: string;
  processor?: {
    class_name: string;
    module: string;
    table_name: string;
    minio_fields?: string[];
  };
  schema: YamlSchema;
  [key: string]: any; // Other stream config fields
}

/**
 * Map YAML types to Drizzle types
 */
function mapTypeToDrizzle(column: YamlColumn): string {
  const baseType = column.type.toLowerCase();
  
  switch (baseType) {
    case 'uuid':
      return 'uuid';
    case 'string':
      return column.max_length ? `varchar({ length: ${column.max_length} })` : 'text';
    case 'text':
      return 'text';
    case 'integer':
      return 'integer';
    case 'smallint':
      return 'smallint';
    case 'bigint':
      return 'bigint({ mode: "number" })';
    case 'float':
      return 'real';
    case 'double':
      return 'doublePrecision';
    case 'decimal':
      return 'decimal';
    case 'boolean':
      return 'boolean';
    case 'timestamp':
      return 'timestamp({ withTimezone: true })';
    case 'date':
      return 'date';
    case 'time':
      return 'time';
    case 'json':
    case 'jsonb':
      return 'jsonb';
    case 'string[]':
      return 'text().array()';
    case 'integer[]':
      return 'integer().array()';
    case 'uuid[]':
      return 'uuid().array()';
    default:
      console.warn(`Unknown type: ${baseType}, defaulting to text`);
      return 'text';
  }
}

/**
 * Convert snake_case to camelCase
 */
function toCamelCase(str: string): string {
  return str.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

/**
 * Generate field definition for a column
 */
function generateFieldDefinition(column: YamlColumn): string {
  const fieldName = toCamelCase(column.name);
  const columnName = column.name;
  const drizzleType = mapTypeToDrizzle(column);
  
  // Build field definition based on type
  let field = '';
  
  if (drizzleType.startsWith('varchar')) {
    // Handle varchar with length
    const lengthMatch = drizzleType.match(/length: (\d+)/);
    if (lengthMatch) {
      field = `  ${fieldName}: varchar('${columnName}', { length: ${lengthMatch[1]} })`;
    } else {
      field = `  ${fieldName}: varchar('${columnName}')`;
    }
  } else if (drizzleType === 'timestamp({ withTimezone: true })') {
    // Handle timestamp with timezone
    field = `  ${fieldName}: timestamp('${columnName}', { withTimezone: true })`;
  } else if (drizzleType === 'bigint({ mode: "number" })') {
    // Handle bigint with mode
    field = `  ${fieldName}: bigint('${columnName}', { mode: 'number' })`;
  } else if (drizzleType.endsWith('.array()')) {
    // Handle array types
    const baseType = drizzleType.replace('.array()', '');
    field = `  ${fieldName}: ${baseType}('${columnName}').array()`;
  } else {
    // Handle simple types
    const baseType = drizzleType.replace('()', '');
    field = `  ${fieldName}: ${baseType}('${columnName}')`;
  }
  
  // Add modifiers
  if (column.nullable === false) {
    field += '.notNull()';
  }
  
  if (column.default !== undefined) {
    if (typeof column.default === 'boolean') {
      field += `.default(${column.default})`;
    } else if (typeof column.default === 'string') {
      field += `.default('${column.default}')`;
    } else {
      field += `.default(${column.default})`;
    }
  }
  
  // Add comment
  if (column.description) {
    field += `,  // ${column.description}`;
  } else {
    field += ',';
  }
  
  return field;
}

/**
 * Generate Drizzle schema from YAML
 */
function generateDrizzleSchema(yamlPath: string, schema: YamlSchema): string {
  let sourceType: string;
  let streamName: string;
  
  if (yamlPath.startsWith('registry:')) {
    // Extract source from stream name in registry (e.g., "mac_apps" -> "mac", "apps")
    const fullStreamName = yamlPath.replace('registry:', '');
    const parts = fullStreamName.split('_');
    sourceType = parts[0];
    streamName = parts.slice(1).join('_') || fullStreamName;
  } else {
    const sourceMatch = yamlPath.match(/sources\/([^/]+)\/([^/]+)\/_stream\.yaml$/);
    if (!sourceMatch) {
      throw new Error(`Invalid path structure: ${yamlPath}`);
    }
    [, sourceType, streamName] = sourceMatch;
  }
  
  const tableName = schema.table_name;
  const tableVarName = toCamelCase(tableName);
  
  // Collect imports based on column types
  const drizzleImports = new Set(['pgTable', 'uuid', 'timestamp']);
  
  // Add index if needed
  if (schema.indexes && schema.indexes.length > 0) {
    drizzleImports.add('index');
  }
  
  // Add imports based on column types
  schema.columns.forEach(col => {
    const drizzleType = mapTypeToDrizzle(col);
    if (drizzleType.includes('varchar')) drizzleImports.add('varchar');
    if (drizzleType.includes('text')) drizzleImports.add('text');
    if (drizzleType.includes('integer')) drizzleImports.add('integer');
    if (drizzleType.includes('real')) drizzleImports.add('real');
    if (drizzleType.includes('boolean')) drizzleImports.add('boolean');
    if (drizzleType.includes('jsonb')) drizzleImports.add('jsonb');
    if (drizzleType.includes('date')) drizzleImports.add('date');
    if (drizzleType.includes('time')) drizzleImports.add('time');
    if (drizzleType.includes('doublePrecision')) drizzleImports.add('doublePrecision');
    if (drizzleType.includes('decimal')) drizzleImports.add('decimal');
    if (drizzleType.includes('smallint')) drizzleImports.add('smallint');
    if (drizzleType.includes('bigint')) drizzleImports.add('bigint');
  });
  
  const sortedImports = Array.from(drizzleImports).sort();
  
  let content = `/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file is generated from: sources/${sourceType}/${streamName}/_stream.yaml (schema section)
 * To modify the schema, edit the source YAML file and regenerate.
 * 
 * Generated at: ${new Date().toISOString()}
 */

import { ${sortedImports.join(', ')} } from 'drizzle-orm/pg-core';
import { sources } from '../sources';

/**
 * ${schema.description}
 */
export const ${tableVarName} = pgTable('${tableName}', {
  // Common fields
  id: uuid('id').primaryKey().defaultRandom(),
  sourceId: uuid('source_id')
    .notNull()
    .references(() => sources.id, { onDelete: 'cascade' }),
  timestamp: timestamp('timestamp', { withTimezone: true }).notNull(),
  
  // Stream-specific fields
`;

  // Add column definitions
  schema.columns.forEach(column => {
    content += '\n' + generateFieldDefinition(column) + ',';
  });
  
  content += `
  
  // Timestamps
  createdAt: timestamp('created_at', { withTimezone: true }).notNull().defaultNow(),
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull().defaultNow(),
}`;

  // Add indexes if defined
  if (schema.indexes && schema.indexes.length > 0) {
    content += `, (table) => ({\n  // Indexes\n`;
    
    schema.indexes.forEach(idx => {
      const indexName = `${tableVarName}${idx.columns.map(c => toCamelCase(c).charAt(0).toUpperCase() + toCamelCase(c).slice(1)).join('')}Idx`;
      const columns = idx.columns.map(c => `table.${toCamelCase(c)}`).join(', ');
      content += `  ${indexName}: index('${tableName}_${idx.columns.join('_')}_idx').on(${columns}),\n`;
    });
    
    // Always add source_id index
    content += `  sourceIdIdx: index('${tableName}_source_id_idx').on(table.sourceId),\n`;
    content += `})`;
  } else {
    // Even without custom indexes, add source_id index
    content += `, (table) => ({
  sourceIdIdx: index('${tableName}_source_id_idx').on(table.sourceId),
})`;
  }

  content += `);\n\n`;

  // Add type exports
  content += `// Type exports\n`;
  content += `export type ${tableVarName.charAt(0).toUpperCase() + tableVarName.slice(1)} = typeof ${tableVarName}.$inferSelect;\n`;
  content += `export type New${tableVarName.charAt(0).toUpperCase() + tableVarName.slice(1)} = typeof ${tableVarName}.$inferInsert;\n`;

  return content;
}

/**
 * Generate index file that re-exports all stream schemas
 */
function generateIndexFile(schemas: Array<{ path: string; schema: YamlSchema }>): string {
  let content = `/**
 * AUTO-GENERATED FILE - DO NOT EDIT
 * 
 * This file re-exports all stream schemas.
 * Generated at: ${new Date().toISOString()}
 */

`;

  schemas.forEach(({ schema }) => {
    const tableName = schema.table_name;
    content += `export * from './${tableName}';\n`;
  });

  return content;
}

/**
 * Main function
 */
async function main() {
  console.log('🔄 Generating Drizzle schemas from YAML registry...\n');
  
  // Read the generated registry
  const registryPath = path.join('sources', '_generated_registry.yaml');
  
  if (!fs.existsSync(registryPath)) {
    console.error(`❌ Registry file not found: ${registryPath}`);
    console.error('Please run: python scripts/generate_registry.py');
    process.exit(1);
  }
  
  const registryContent = fs.readFileSync(registryPath, 'utf8');
  const registry = yaml.load(registryContent) as any;
  
  if (!registry.streams) {
    console.error('❌ No streams found in registry');
    process.exit(1);
  }
  
  const streamEntries = Object.entries(registry.streams);
  console.log(`Found ${streamEntries.length} streams in registry\n`);
  
  const schemas: Array<{ path: string; schema: YamlSchema }> = [];
  
  for (const [streamName, streamData] of streamEntries) {
    try {
      const stream = streamData as any;
      console.log(`Processing: ${streamName}`);
      
      // Extract schema section
      if (!stream.schema) {
        console.log(`  ⚠️  No schema section found for ${streamName}`);
        continue;
      }
      
      const schema = stream.schema;
      schemas.push({ path: `registry:${streamName}`, schema });
      
      // Generate Drizzle schema
      const drizzleSchema = generateDrizzleSchema(`registry:${streamName}`, schema);
      
      // Write to file
      const outputDir = path.join('apps', 'web', 'src', 'lib', 'db', 'schema', 'generated');
      const outputPath = path.join(outputDir, `${schema.table_name}.ts`);
      
      // Ensure directory exists
      fs.mkdirSync(outputDir, { recursive: true });
      
      // Write file
      fs.writeFileSync(outputPath, drizzleSchema);
      console.log(`  ✅ Generated: ${outputPath}`);
      
    } catch (error) {
      console.error(`  ❌ Error processing ${streamName}:`, error);
    }
  }
  
  // Generate index file
  if (schemas.length > 0) {
    const indexContent = generateIndexFile(schemas);
    const indexPath = path.join('apps', 'web', 'src', 'lib', 'db', 'schema', 'generated', 'index.ts');
    fs.writeFileSync(indexPath, indexContent);
    console.log(`\n✅ Generated index file: ${indexPath}`);
  }
  
  console.log('\n✨ Drizzle schema generation complete!');
  console.log('\nNext steps:');
  console.log('1. Review the generated schemas in apps/web/src/lib/db/schema/generated/');
  console.log('2. Run: cd apps/web && pnpm drizzle-kit generate');
  console.log('3. Run: cd apps/web && pnpm drizzle-kit push');
}

// Run the script
main().catch(console.error);