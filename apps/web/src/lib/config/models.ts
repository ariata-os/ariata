/**
 * Model configuration fetched from database via API
 * This provides a single source of truth for model metadata including context window limits
 */

import { browser } from '$app/environment';

export interface ModelOption {
	id: string;
	displayName: string;
	provider: string;
	contextWindow: number | null;
	maxOutputTokens: number | null;
	supportsTools: boolean | null;
	enabled: boolean;
	sortOrder: number;
	isDefault?: boolean;
}

/**
 * Fetch all models from API
 */
export async function fetchModels(): Promise<ModelOption[]> {
	if (!browser) {
		return [];
	}

	const response = await fetch('/api/models');
	if (!response.ok) {
		throw new Error(`Failed to fetch models: ${response.statusText}`);
	}
	const data = await response.json();

	// Transform API response to ModelOption format
	return data.map((model: any) => ({
		id: model.model_id,
		displayName: model.display_name,
		provider: model.provider,
		contextWindow: model.context_window,
		maxOutputTokens: model.max_output_tokens,
		supportsTools: model.supports_tools,
		enabled: model.enabled,
		sortOrder: model.sort_order,
		isDefault: model.is_default || false
	}));
}

/**
 * Get model configuration by ID from API
 */
export async function getModelById(modelId: string): Promise<ModelOption | null> {
	if (!browser) {
		return null;
	}

	const response = await fetch(`/api/models/${encodeURIComponent(modelId)}`);
	if (!response.ok) {
		if (response.status === 404) return null;
		throw new Error(`Failed to fetch model: ${response.statusText}`);
	}
	const model = await response.json();

	return {
		id: model.model_id,
		displayName: model.display_name,
		provider: model.provider,
		contextWindow: model.context_window,
		maxOutputTokens: model.max_output_tokens,
		supportsTools: model.supports_tools,
		enabled: model.enabled,
		sortOrder: model.sort_order,
		isDefault: model.is_default || false
	};
}
