/**
 * Tool type definitions for AI SDK v6
 */
import type { z } from 'zod';

/**
 * AI SDK v6 tool format - plain object with description, inputSchema, execute
 */
export interface AiSdkTool {
	description: string;
	inputSchema: z.ZodType<any, any>;
	execute: (args: any) => Promise<any>;
}

/**
 * Tool registry - maps tool names to their implementations
 */
export type ToolRegistry = Record<string, AiSdkTool>;

/**
 * Tool metadata for UI and documentation
 */
export interface ToolMetadata {
	name: string;
	description: string;
}
