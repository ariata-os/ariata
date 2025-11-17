<script lang="ts">
	import { Page } from "$lib";
	import type { ModelOption } from "$lib/config/models";
	import { getModels } from "$lib/stores/models.svelte";
	import { getEnabledAgents } from "$lib/config/agents";
	import "iconify-icon";
	import { onMount } from "svelte";

	// Tool type from API
	interface Tool {
		id: string;
		name: string;
		description: string | null;
		category: string | null;
		icon: string | null;
		is_pinnable: boolean;
		display_order: number | null;
	}

	let loading = $state(true);
	let saving = $state(false);
	let saveSuccess = $state(false);

	// Get models from store
	const models = $derived(getModels());
	const DEFAULT_MODEL = $derived(models.find(m => m.isDefault) || models[0]);

	// Assistant profile fields
	let assistantName = $state("");
	let defaultAgentId = $state("auto");
	let defaultModelId = $state("");
	let selectedModel: ModelOption | undefined = $state(undefined);
	let enabledTools: Record<string, boolean> = $state({});
	let pinnedToolIds: string[] = $state([]);
	let availablePinnableTools: Array<{ id: string; name: string; description: string | null; icon: string | null }> = $state([]);

	// UI preferences
	let contextIndicatorAlwaysVisible = $state(false);
	let contextIndicatorShowThreshold = $state(70);

	// Tools fetched from API
	let tools: Tool[] = $state([]);

	// Get available agents (simplified 2-agent system)
	const agents = [
		{
			id: "auto",
			name: "Auto (Recommended)",
			description: "Automatically selects the best agent",
		},
		...getEnabledAgents(),
	];

	onMount(async () => {
		await loadProfile();
	});

	async function loadProfile() {
		loading = true;
		try {
			console.log("[Assistant Settings] Fetching assistant profile...");

			// Fetch profile and tools in parallel
			const [profileResponse, toolsResponse] = await Promise.all([
				fetch("/api/assistant-profile"),
				fetch("/api/tools")
			]);

			console.log(
				"[Assistant Settings] Profile response status:",
				profileResponse.status,
			);
			console.log(
				"[Assistant Settings] Tools response status:",
				toolsResponse.status,
			);

			if (profileResponse.ok) {
				const profile = await profileResponse.json();
				console.log("[Assistant Settings] Profile loaded:", profile);

				// Populate fields from profile
				assistantName = profile.assistant_name || "";
				defaultAgentId = profile.default_agent_id || "auto";
				defaultModelId = profile.default_model_id || DEFAULT_MODEL?.id || "";
				enabledTools = profile.enabled_tools || {};
				pinnedToolIds = profile.pinned_tool_ids || [];

				// Load UI preferences
				if (profile.ui_preferences?.contextIndicator) {
					contextIndicatorAlwaysVisible = profile.ui_preferences.contextIndicator.alwaysVisible ?? false;
					contextIndicatorShowThreshold = profile.ui_preferences.contextIndicator.showThreshold ?? 70;
				}

				// Update selected model
				const foundModel = models.find((m) => m.id === defaultModelId);
				if (foundModel) {
					selectedModel = foundModel;
				} else if (DEFAULT_MODEL) {
					selectedModel = DEFAULT_MODEL;
				}
				console.log(
					"[Assistant Settings] Fields populated successfully",
				);
			} else {
				console.error(
					"[Assistant Settings] Failed to load profile, status:",
					profileResponse.status,
				);
			}

			if (toolsResponse.ok) {
				tools = await toolsResponse.json();
				console.log("[Assistant Settings] Loaded", tools.length, "tools");

				// Ensure all tools have explicit enabled/disabled values
				// Default to true if not set
				for (const tool of tools) {
					if (enabledTools[tool.id] === undefined) {
						enabledTools[tool.id] = true;
					}
				}

				// Filter pinnable tools from the loaded tools
				availablePinnableTools = tools.filter(t => t.is_pinnable);
			} else {
				console.error(
					"[Assistant Settings] Failed to load tools, status:",
					toolsResponse.status,
				);
			}
		} catch (error) {
			console.error("Failed to load assistant profile:", error);
		} finally {
			loading = false;
			console.log("[Assistant Settings] Loading complete");
		}
	}

	function togglePinnedTool(toolId: string) {
		if (pinnedToolIds.includes(toolId)) {
			pinnedToolIds = pinnedToolIds.filter(id => id !== toolId);
		} else {
			pinnedToolIds = [...pinnedToolIds, toolId];
		}
	}

	async function saveProfile() {
		saving = true;
		saveSuccess = false;
		try {
			const response = await fetch("/api/assistant-profile", {
				method: "PUT",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({
					assistant_name: assistantName || null,
					default_agent_id: defaultAgentId || null,
					default_model_id: defaultModelId || null,
					enabled_tools: enabledTools,
					pinned_tool_ids: pinnedToolIds,
					ui_preferences: {
						contextIndicator: {
							alwaysVisible: contextIndicatorAlwaysVisible,
							showThreshold: contextIndicatorShowThreshold,
						},
					},
				}),
			});

			if (response.ok) {
				saveSuccess = true;
				setTimeout(() => {
					saveSuccess = false;
				}, 3000);
			} else {
				throw new Error("Failed to save assistant profile");
			}
		} catch (error) {
			console.error("Failed to save assistant profile:", error);
			alert("Failed to save assistant profile. Please try again.");
		} finally {
			saving = false;
		}
	}
</script>

<Page>
	<div class="max-w-3xl">
		<!-- Header -->
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">
				Assistant Settings
			</h1>
			<p class="text-neutral-600">
				Customize your AI assistant preferences
			</p>
		</div>

		{#if loading}
			<div class="text-center py-12 text-neutral-500">
				Loading settings...
			</div>
		{:else}
			<form onsubmit={(e) => { e.preventDefault(); saveProfile(); }} class="space-y-6">
				<!-- Pinned Tools Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Pinned Tools
					</h2>
					<p class="text-sm text-neutral-600 mb-6">
						Select tools to pin to your chat interface. Pinned tools appear as quick-access buttons when starting a new conversation.
					</p>

					<div class="space-y-3">
						{#each availablePinnableTools as tool}
							<label class="flex items-start gap-3 cursor-pointer group">
								<input
									type="checkbox"
									checked={pinnedToolIds.includes(tool.id)}
									onchange={() => togglePinnedTool(tool.id)}
									class="mt-0.5 w-4 h-4 border-neutral-300 rounded text-neutral-900 focus:ring-2 focus:ring-neutral-900 cursor-pointer"
								/>
								<div class="flex-1 min-w-0 flex items-center gap-2">
									{#if tool.icon}
										<iconify-icon icon={tool.icon} width="16" class="text-neutral-500"></iconify-icon>
									{/if}
									<div class="flex-1">
										<div class="text-sm font-medium text-neutral-900 group-hover:text-neutral-700">
											{tool.name}
										</div>
										{#if tool.description}
											<div class="text-xs text-neutral-600 mt-0.5">
												{tool.description}
											</div>
										{/if}
									</div>
								</div>
							</label>
						{/each}
					</div>

					<div class="mt-4 p-3 bg-neutral-50 border border-neutral-200 rounded-md">
						<p class="text-xs text-neutral-600">
							<strong>Tip:</strong> Pinned tools appear as quick-access buttons below the chat input when starting a new conversation. Click them to instantly execute the tool.
						</p>
					</div>
				</div>

				<!-- Display Preferences Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Display Preferences
					</h2>
					<p class="text-sm text-neutral-600 mb-6">
						Customize how the context indicator is displayed in the chat interface.
					</p>

					<div class="space-y-4">
						<!-- Always visible toggle -->
						<label class="flex items-start gap-3 cursor-pointer group">
							<input
								type="checkbox"
								bind:checked={contextIndicatorAlwaysVisible}
								class="mt-0.5 w-4 h-4 border-neutral-300 rounded text-neutral-900 focus:ring-2 focus:ring-neutral-900 cursor-pointer"
							/>
							<div class="flex-1">
								<div class="text-sm font-medium text-neutral-900 group-hover:text-neutral-700">
									Always show context indicator
								</div>
								<div class="text-xs text-neutral-600 mt-0.5">
									When enabled, the context indicator will always be visible. When disabled, it will only appear when usage exceeds the threshold below.
								</div>
							</div>
						</label>

						<!-- Threshold input -->
						<div>
							<label
								for="contextThreshold"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Show warning at threshold (%)
							</label>
							<input
								type="number"
								id="contextThreshold"
								bind:value={contextIndicatorShowThreshold}
								min="0"
								max="100"
								class="w-32 px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
							/>
							<p class="text-xs text-neutral-500 mt-1">
								The context indicator will turn yellow when usage exceeds this percentage (default: 70%)
							</p>
						</div>
					</div>

					<div class="mt-4 p-3 bg-neutral-50 border border-neutral-200 rounded-md">
						<p class="text-xs text-neutral-600">
							<strong>Tip:</strong> The context indicator shows how much of your model's context window is being used. It helps you know when to start a new conversation to avoid running out of space.
						</p>
					</div>
				</div>

				<!-- Tool & Widget Preferences Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Tool & Widget Preferences
					</h2>
					<p class="text-sm text-neutral-600 mb-6">
						Enable or disable specific tools and widgets. This is useful to prevent conflicts with MCP integrations (e.g., disable the Pursuits widget if using Todoist MCP).
					</p>

					<div class="space-y-3">
						{#each tools as tool}
							<label class="flex items-start gap-3 cursor-pointer group">
								<input
									type="checkbox"
									bind:checked={enabledTools[tool.id]}
									class="mt-0.5 w-4 h-4 border-neutral-300 rounded text-neutral-900 focus:ring-2 focus:ring-neutral-900 cursor-pointer"
								/>
								<div class="flex-1 min-w-0 flex items-center gap-2">
									{#if tool.icon}
										<iconify-icon icon={tool.icon} width="16" class="text-neutral-500"></iconify-icon>
									{/if}
									<div class="flex-1">
										<div class="text-sm font-medium text-neutral-900 group-hover:text-neutral-700">
											{tool.name}
										</div>
										{#if tool.description}
											<div class="text-xs text-neutral-600 mt-0.5">
												{tool.description}
											</div>
										{/if}
									</div>
								</div>
							</label>
						{/each}
					</div>

					<div class="mt-4 p-3 bg-neutral-50 border border-neutral-200 rounded-md">
						<p class="text-xs text-neutral-600">
							<strong>Note:</strong> Disabling a tool will prevent the assistant from using it in all conversations. This is useful to prevent conflicts with MCP integrations (e.g., disable the Pursuits widget if using Todoist MCP).
						</p>
					</div>
				</div>

				<!-- Assistant Name Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Assistant Name
					</h2>
					<div class="space-y-4">
						<div>
							<label
								for="assistantName"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Name
							</label>
							<input
								type="text"
								id="assistantName"
								bind:value={assistantName}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
								placeholder="Assistant"
							/>
							<p class="text-xs text-neutral-500 mt-1">
								Give your AI assistant a personalized name
								(e.g., "Aria", "Alex", "Claude")
							</p>
						</div>
					</div>
				</div>

				<!-- Default Agent Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Default Agent
					</h2>
					<div class="space-y-4">
						<div>
							<label
								for="defaultAgent"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Agent
							</label>
							<select
								id="defaultAgent"
								bind:value={defaultAgentId}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
							>
								{#each agents as agent}
									<option value={agent.id}
										>{agent.name}</option
									>
								{/each}
							</select>
							<p class="text-xs text-neutral-500 mt-2">
								This agent will be used by default for new
								conversations. You can change it per
								conversation.
							</p>
						</div>
					</div>
				</div>

				<!-- Default Model Section -->
				<div class="bg-white border border-neutral-200 rounded-lg p-6">
					<h2 class="text-lg font-medium text-neutral-900 mb-4">
						Default Model
					</h2>
					<div class="space-y-4">
						<div>
							<label
								for="defaultModel"
								class="block text-sm font-medium text-neutral-700 mb-2"
							>
								Model
							</label>
							<select
								id="defaultModel"
								bind:value={defaultModelId}
								class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
							>
								{#each models as model}
									<option value={model.id}
										>{model.displayName}</option
									>
								{/each}
							</select>
							<p class="text-xs text-neutral-500 mt-2">
								This language model will be used by default. You
								can override it per conversation.
							</p>
						</div>
					</div>
				</div>

				<!-- Save Button -->
				<div class="flex items-center gap-3 pt-2">
					<button
						type="submit"
						disabled={saving}
						class="px-6 py-2 bg-neutral-900 text-white rounded-md hover:bg-neutral-800 focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
					>
						{saving ? "Saving..." : "Save Settings"}
					</button>
					{#if saveSuccess}
						<span
							class="text-sm text-green-600 flex items-center gap-1"
						>
							<iconify-icon icon="mdi:check-circle" width="16"
							></iconify-icon>
							Saved successfully
						</span>
					{/if}
				</div>
			</form>
		{/if}
	</div>
</Page>
