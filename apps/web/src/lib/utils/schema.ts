import { zodToJsonSchema as zodToJsonSchemaLib } from 'zod-to-json-schema';
import { z } from 'zod';

/**
 * Convert Zod schema to clean JSON Schema for Anthropic API
 *
 * The AI SDK's automatic Zod conversion can sometimes produce schemas
 * with extra metadata that Anthropic rejects. This function manually
 * converts Zod → JSON Schema and removes unsupported fields.
 */
export const zodToJsonSchema = <T extends z.ZodTypeAny>(zodSchema: T): any => {
	const jsonSchema = zodToJsonSchemaLib(zodSchema, {
		target: 'jsonSchema7',
		$refStrategy: 'none'
	});

	// Remove fields that Anthropic API doesn't support
	const cleaned = JSON.parse(JSON.stringify(jsonSchema));
	delete cleaned.$schema;
	delete cleaned.additionalProperties;

	return cleaned;
};

/**
 * Pass-through for Zod schemas to AI SDK tools
 *
 * The AI SDK (v5) handles Zod schemas natively. We simply return
 * the raw Zod schema and let the AI SDK convert it internally.
 *
 * Usage:
 * ```typescript
 * tool({
 *   description: '...',
 *   parameters: zodSchemaForTools(z.object({
 *     field: z.string().describe('...')
 *   })),
 *   execute: async ({ field }) => { ... }
 * })
 * ```
 */
export const zodSchemaForTools = <T extends z.ZodTypeAny>(zodSchema: T) => {
	// Return raw Zod - AI SDK will convert it internally
	return zodSchema;
};
