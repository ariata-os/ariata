#!/usr/bin/env tsx
import { Project, SourceFile, Node, SyntaxKind, Type } from 'ts-morph';
import * as fs from 'fs';
import * as path from 'path';

interface ColumnDef {
  name: string;
  type: string;
  nullable: boolean;
  primaryKey: boolean;
  isArray?: boolean;  // Track if this is an array type
  defaultValue?: string;
  foreignKey?: { table: string; column: string; onDelete?: string };
  enumName?: string;  // Track the original enum name
}

interface TableDef {
  name: string;
  columns: ColumnDef[];
}

// Type mapping from Drizzle types to Python types
const typeToPython: Record<string, string> = {
  'uuid': 'UUID',
  'varchar': 'str',
  'text': 'str',
  'boolean': 'bool',
  'integer': 'int',
  'real': 'float',
  'timestamp': 'datetime',
  'json': 'Dict[str, Any]',
  'jsonb': 'Dict[str, Any]',
  'pgEnum': 'str',  // Enums as strings for now
  'ingestionStatusEnum': 'str',  // Specific enum
};

interface ExtractedData {
  tables: TableDef[];
  enums: Array<{ name: string; values: string[] }>;
}

function extractDrizzleSchemas(schemaPath: string): ExtractedData {
  const project = new Project({
    tsConfigFilePath: path.join(process.cwd(), 'tsconfig.json'),
  });
  
  const sourceFile = project.addSourceFileAtPath(schemaPath);
  const tables: TableDef[] = [];
  const enums: Array<{ name: string; values: string[] }> = [];
  
  // Find all pgEnum calls
  sourceFile.forEachDescendant((node) => {
    if (Node.isCallExpression(node)) {
      const expression = node.getExpression();
      if (Node.isIdentifier(expression) && expression.getText() === 'pgEnum') {
        const args = node.getArguments();
        if (args.length >= 2) {
          const enumName = args[0].getText().replace(/['"]/g, '');
          const valuesArray = args[1];
          if (Node.isArrayLiteralExpression(valuesArray)) {
            const values = valuesArray.getElements()
              .map(e => e.getText().replace(/['"]/g, ''));
            enums.push({ name: enumName, values });
          }
        }
      }
    }
  });
  
  // Find all pgTable calls
  sourceFile.forEachDescendant((node) => {
    if (Node.isCallExpression(node)) {
      const expression = node.getExpression();
      if (Node.isIdentifier(expression) && expression.getText() === 'pgTable') {
        const table = extractTableInfo(node);
        if (table) {
          tables.push(table);
        }
      }
    }
  });
  
  return { tables, enums };
}

function extractTableInfo(tableCall: any): TableDef | null {
  const args = tableCall.getArguments();
  if (args.length < 2) return null;
  
  // Get table name
  const tableName = args[0].getText().replace(/['"]/g, '');
  
  // Get columns
  const columnsObj = args[1];
  const columns: ColumnDef[] = [];
  
  if (Node.isObjectLiteralExpression(columnsObj)) {
    columnsObj.getProperties().forEach((prop: any) => {
      if (Node.isPropertyAssignment(prop)) {
        const column = extractColumnInfo(prop);
        if (column) {
          columns.push(column);
        }
      }
    });
  }
  
  return { name: tableName, columns };
}

function extractColumnInfo(prop: any): ColumnDef | null {
  const name = prop.getName();
  const init = prop.getInitializer();
  
  if (!init || !Node.isCallExpression(init)) return null;
  
  let type = 'varchar';
  let nullable = true;
  let primaryKey = false;
  let isArray = false;
  let defaultValue: string | undefined;
  let foreignKey: ColumnDef['foreignKey'];
  
  // Walk through the call chain to extract all information
  const calls: string[] = [];
  let current = init;
  
  // Collect all method calls in the chain
  while (current && Node.isCallExpression(current)) {
    const expr = current.getExpression();
    if (Node.isPropertyAccessExpression(expr)) {
      calls.push(expr.getName());
      current = expr.getExpression();
    } else if (Node.isIdentifier(expr)) {
      calls.push(expr.getText());
      break;
    } else {
      break;
    }
  }
  
  // The base type is the last call in our reverse walk
  if (calls.length > 0) {
    type = calls[calls.length - 1];
  }
  
  // Process modifiers (in reverse order since we walked backwards)
  for (let i = calls.length - 2; i >= 0; i--) {
    const modifier = calls[i];
    
    switch (modifier) {
      case 'array':
        isArray = true;
        break;
      case 'notNull':
        nullable = false;
        break;
      case 'primaryKey':
        primaryKey = true;
        nullable = false;
        break;
      case 'defaultNow':
        defaultValue = 'datetime.now';
        break;
      case 'defaultRandom':
        defaultValue = 'uuid4';
        break;
      case 'default':
        // Need to get the argument from the original call
        let callNode = init;
        for (let j = 0; j <= i; j++) {
          if (Node.isCallExpression(callNode) && Node.isPropertyAccessExpression(callNode.getExpression())) {
            if (callNode.getExpression().getName() === 'default') {
              const args = callNode.getArguments();
              if (args.length > 0) {
                const argText = args[0].getText();
                if (argText === 'true' || argText === 'false') {
                  defaultValue = argText.charAt(0).toUpperCase() + argText.slice(1);
                } else {
                  defaultValue = argText;
                }
              }
            }
          }
          if (Node.isPropertyAccessExpression(callNode.getExpression())) {
            callNode = callNode.getExpression().getExpression();
          }
        }
        break;
      case 'references':
        // Extract foreign key reference
        let refNode = init;
        for (let j = 0; j <= i; j++) {
          if (Node.isCallExpression(refNode) && Node.isPropertyAccessExpression(refNode.getExpression())) {
            if (refNode.getExpression().getName() === 'references') {
              const args = refNode.getArguments();
              if (args.length > 0 && Node.isArrowFunction(args[0])) {
                const body = args[0].getBody();
                if (Node.isPropertyAccessExpression(body)) {
                  const tableName = body.getExpression().getText().replace('() => ', '');
                  const columnName = body.getName();
                  foreignKey = { table: tableName, column: columnName };
                }
                // Check for onDelete option
                if (args.length > 1 && Node.isObjectLiteralExpression(args[1])) {
                  const deleteOpt = args[1].getProperty('onDelete');
                  if (deleteOpt && Node.isPropertyAssignment(deleteOpt)) {
                    const deleteValue = deleteOpt.getInitializer()?.getText().replace(/['"]/g, '');
                    if (foreignKey) foreignKey.onDelete = deleteValue;
                  }
                }
              }
            }
          }
          if (Node.isPropertyAccessExpression(refNode.getExpression())) {
            refNode = refNode.getExpression().getExpression();
          }
        }
        break;
    }
  }
  
  // Clean up the type
  type = type.replace(/\(\).*/, '').trim();
  
  // Check if this is an enum type
  let enumName: string | undefined;
  // Check if the type contains 'Enum' (handles cases like ingestionStatusEnum)
  if (type.toLowerCase().includes('enum')) {
    enumName = type;
  }
  
  return {
    name,
    type: typeToPython[type] || 'Any',
    nullable,
    primaryKey,
    isArray,
    defaultValue,
    foreignKey,
    enumName,
  };
}

function generateSQLModel(table: TableDef, enums: Array<{ name: string; values: string[] }> = []): string {
  const imports = new Set<string>([
    'from sqlalchemy import Column, String, Boolean, Integer, Float, DateTime, Text, ForeignKey, JSON',
    'from sqlalchemy.dialects.postgresql import UUID, ARRAY',
    'from sources.base.models.base import Base',
    'from datetime import datetime',
  ]);
  
  // Map enum names to their Python class names
  const enumNameMap: Record<string, string> = {};
  enums.forEach(enumDef => {
    let enumClassName = enumDef.name
      .split('_')
      .map(part => part.charAt(0).toUpperCase() + part.slice(1))
      .join('')
      .replace(/Enum$/, '')
      .replace(/Status$/, '');
    if (!enumClassName.endsWith('Status')) {
      enumClassName += 'Status';
    }
    enumNameMap[enumDef.name] = enumClassName;
    
    // Create camelCase version: ingestion_status -> ingestionStatusEnum
    const parts = enumDef.name.split('_');
    const camelCaseName = parts[0] + parts.slice(1).map(p => p.charAt(0).toUpperCase() + p.slice(1)).join('') + 'Enum';
    enumNameMap[camelCaseName] = enumClassName;
  });
  
  // Check what imports we need
  const hasUUID = table.columns.some(c => c.type === 'UUID');
  const hasDict = table.columns.some(c => c.type === 'Dict[str, Any]');
  const hasOptional = table.columns.some(c => c.nullable);
  const hasForeignKey = table.columns.some(c => c.foreignKey);
  const hasEnum = enums.length > 0;
  const hasEnumColumn = table.columns.some(c => c.enumName);
  
  if (hasUUID) {
    imports.add('from uuid import uuid4');
  }
  if (hasDict || hasOptional) {
    imports.add('from typing import Dict, Any, Optional');
  }
  // Already imported above
  if (hasEnum) {
    imports.add('from enum import Enum');
  }
  if (hasEnumColumn) {
    imports.add('from sqlalchemy.dialects.postgresql import ENUM as PGEnum');
  }
  
  // Convert table name to PascalCase
  const className = table.name
    .split('_')
    .map(part => part.charAt(0).toUpperCase() + part.slice(1))
    .join('');
  
  // Type mapping for SQLAlchemy columns
  const typeToSQLAlchemy: Record<string, string> = {
    'UUID': 'UUID(as_uuid=True)',
    'str': 'String',
    'bool': 'Boolean',
    'int': 'Integer',
    'float': 'Float',
    'datetime': 'DateTime',
    'Dict[str, Any]': 'JSON',
    'Any': 'JSON',  // Map Any type to JSON
  };
  
  // Generate column definitions
  const columnDefs = table.columns.map(col => {
    const columnArgs: string[] = [];
    
    // Convert camelCase to snake_case for Python
    const pythonName = col.name.replace(/([A-Z])/g, '_$1').toLowerCase().replace(/^_/, '');
    
    // Get SQLAlchemy type
    let sqlAlchemyType = typeToSQLAlchemy[col.type] || 'String';
    
    // Handle enum columns
    if (col.enumName) {
      let enumClassName = enumNameMap[col.enumName] || enumNameMap[col.enumName.replace('Enum', '')];
      if (enumClassName) {
        const pgEnumName = col.enumName.replace(/Enum$/, '').replace(/([A-Z])/g, '_$1').toLowerCase().replace(/^_/, '');
        sqlAlchemyType = `PGEnum(${enumClassName}, name='${pgEnumName}')`;
      }
    }
    
    // Handle array columns
    if (col.isArray) {
      // For UUID arrays, we need to use UUID type inside ARRAY
      if (col.type === 'UUID') {
        sqlAlchemyType = 'ARRAY(UUID(as_uuid=True))';
      } else if (col.type === 'str') {
        sqlAlchemyType = 'ARRAY(Text)';
      } else {
        // Map the inner type
        const innerType = typeToSQLAlchemy[col.type] || 'String';
        sqlAlchemyType = `ARRAY(${innerType})`;
      }
    }
    
    columnArgs.push(sqlAlchemyType);
    
    // Handle foreign keys
    if (col.foreignKey) {
      const fkTable = col.foreignKey.table;
      const onDelete = col.foreignKey.onDelete || 'CASCADE';
      columnArgs.push(`ForeignKey('${fkTable}.${col.foreignKey.column}', ondelete='${onDelete.toUpperCase()}')`);
    }
    
    // Handle primary key
    if (col.primaryKey) {
      columnArgs.push('primary_key=True');
    }
    
    // Handle nullable
    if (!col.nullable) {
      columnArgs.push('nullable=False');
    }
    
    // Handle defaults
    if (col.defaultValue && !col.primaryKey) {
      if (col.defaultValue === 'datetime.now') {
        columnArgs.push('default=datetime.now');
      } else if (col.defaultValue === 'uuid4') {
        columnArgs.push('default=uuid4');
      } else if (col.defaultValue === 'True' || col.defaultValue === 'False') {
        columnArgs.push(`default=${col.defaultValue}`);
      } else if (col.defaultValue.match(/^[0-9.]+$/)) {
        columnArgs.push(`default=${col.defaultValue}`);
      } else if (col.enumName) {
        // Handle enum defaults
        let enumClassName = enumNameMap[col.enumName] || enumNameMap[col.enumName.replace('Enum', '')];
        if (enumClassName) {
          const cleanDefaultValue = col.defaultValue.replace(/["']/g, '');
          columnArgs.push(`default=${enumClassName}.${cleanDefaultValue.toUpperCase()}`);
        }
      } else {
        // Remove quotes from default value if they exist
        const cleanDefault = col.defaultValue.replace(/^['"]|['"]$/g, '');
        columnArgs.push(`default='${cleanDefault}'`);
      }
    }
    
    // Add index if foreign key
    if (col.foreignKey) {
      columnArgs.push('index=True');
    }
    
    return `    ${pythonName} = Column(${columnArgs.join(', ')})`;
  });
  
  // Generate enum classes
  const enumDefs = enums.map(enumDef => {
    // Convert enum name to PascalCase class name
    let enumClassName = enumDef.name
      .split('_')
      .map(part => part.charAt(0).toUpperCase() + part.slice(1))
      .join('');
    
    // Remove redundant suffixes
    enumClassName = enumClassName
      .replace(/Enum$/, '')
      .replace(/Status$/, '');
    
    // Add Status suffix if not present
    if (!enumClassName.endsWith('Status')) {
      enumClassName += 'Status';
    }
    
    const enumValues = enumDef.values
      .map(v => `    ${v.toUpperCase()} = "${v}"`)
      .join('\n');
    
    return `class ${enumClassName}(str, Enum):
    """${enumDef.name} enum values."""
${enumValues}`;
  });
  
  const enumSection = enumDefs.length > 0 ? '\n\n' + enumDefs.join('\n\n') + '\n\n' : '';
  
  return `"""Generated from Drizzle schema - DO NOT EDIT MANUALLY"""
${Array.from(imports).sort().join('\n')}
${enumSection}

class ${className}(Base):
    """Auto-generated from Drizzle schema."""
    
    __tablename__ = "${table.name}"
    
${columnDefs.join('\n')}
`;
}

// Main execution
async function main() {
  const schemaDir = path.join(process.cwd(), 'src/lib/db/schema');
  const outputDir = path.join(process.cwd(), '../../sources/base/generated_models');
  
  // Create output directory
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }
  
  // Process all schema files
  const schemaFiles = fs.readdirSync(schemaDir)
    .filter(f => f.endsWith('.ts') && f !== 'index.ts');
  
  const allEnums: Array<{ name: string; values: string[]; fileName: string }> = [];
  
  for (const file of schemaFiles) {
    console.log(`Processing ${file}...`);
    const { tables, enums } = extractDrizzleSchemas(path.join(schemaDir, file));
    
    // Track enums for later generation
    enums.forEach(e => allEnums.push({ ...e, fileName: file.replace('.ts', '') }));
    
    for (const table of tables) {
      const pythonCode = generateSQLModel(table, enums);
      const outputPath = path.join(outputDir, `${table.name}.py`);
      fs.writeFileSync(outputPath, pythonCode);
      console.log(`  Generated ${outputPath}`);
    }
  }
  
  // Create __init__.py
  const initContent = schemaFiles
    .map(f => f.replace('.ts', ''))
    .map(f => `from .${f} import *`)
    .join('\n');
  
  fs.writeFileSync(path.join(outputDir, '__init__.py'), initContent + '\n');
  console.log('Done!');
}

main().catch(console.error);