/**
 * Models store - fetches and caches model data from API
 * Centralizes model state management including selection
 */
import { fetchModels, type ModelOption } from '$lib/config/models';

let modelsCache: ModelOption[] = $state([]);
let loading = $state(true);
let error = $state<string | null>(null);
let selectedModel = $state<ModelOption | undefined>(undefined);
let initializationPromise: Promise<void> | null = null;

/**
 * Load models from API
 */
async function loadModels() {
	if (modelsCache.length > 0) {
		// Already loaded
		return;
	}

	loading = true;
	error = null;

	try {
		modelsCache = await fetchModels();
		console.log('[models.svelte.ts] Loaded models:', modelsCache.length);
	} catch (err) {
		error = err instanceof Error ? err.message : 'Failed to load models';
		console.error('Failed to load models:', err);
	} finally {
		loading = false;
	}
}

/**
 * Get initialization promise to wait for models to load
 */
export function getInitializationPromise(): Promise<void> {
	if (!initializationPromise) {
		initializationPromise = loadModels();
	}
	return initializationPromise;
}

/**
 * Get all models (returns cached value, does not trigger load)
 */
export function getModels(): ModelOption[] {
	return modelsCache;
}

/**
 * Get model by ID
 */
export function getModelById(modelId: string): ModelOption | undefined {
	return modelsCache.find((m) => m.id === modelId);
}

/**
 * Get the default model (marked with is_default flag or first in list)
 */
export function getDefaultModel(): ModelOption | undefined {
	const defaultModel = modelsCache.find((m) => m.isDefault);
	return defaultModel || modelsCache[0];
}

/**
 * Get the currently selected model
 */
export function getSelectedModel(): ModelOption | undefined {
	return selectedModel;
}

/**
 * Set the selected model
 */
export function setSelectedModel(model: ModelOption | undefined) {
	selectedModel = model;
	console.log('[models.svelte.ts] Selected model:', model?.displayName);
}

/**
 * Initialize selected model with fallback chain:
 * 1. Conversation model (if exists)
 * 2. Assistant profile default (if exists)
 * 3. is_default flag model
 * 4. First model in list
 */
export function initializeSelectedModel(
	conversationModelId?: string,
	profileDefaultModelId?: string
): void {
	if (selectedModel) {
		// Already initialized
		return;
	}

	// Try conversation model first
	if (conversationModelId) {
		const foundModel = getModelById(conversationModelId);
		if (foundModel) {
			setSelectedModel(foundModel);
			return;
		}
	}

	// Try profile default
	if (profileDefaultModelId) {
		const foundModel = getModelById(profileDefaultModelId);
		if (foundModel) {
			setSelectedModel(foundModel);
			return;
		}
	}

	// Fall back to default model (is_default flag or first model)
	setSelectedModel(getDefaultModel());
}

/**
 * Check if models are loading
 */
export function isLoading(): boolean {
	return loading;
}

/**
 * Get error if any
 */
export function getError(): string | null {
	return error;
}

/**
 * Force reload models
 */
export async function reloadModels() {
	modelsCache = [];
	initializationPromise = null;
	await loadModels();
}
