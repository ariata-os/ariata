<script lang="ts">
	import { Page } from "$lib";
	import "iconify-icon";
	import type { PageData } from "./$types";

	export let data: PageData;

	// Domain to icon and color mapping
	const domainConfig: Record<string, { icon: string; color: string }> = {
		Health: { icon: "ri:heart-pulse-line", color: "text-red-600" },
		Location: { icon: "ri:map-pin-line", color: "text-blue-600" },
		Social: { icon: "ri:chat-3-line", color: "text-purple-600" },
		Activity: { icon: "ri:calendar-event-line", color: "text-green-600" },
		Finance: {
			icon: "ri:money-dollar-circle-line",
			color: "text-emerald-600",
		},
		Ambient: { icon: "ri:cloud-line", color: "text-cyan-600" },
		Knowledge: { icon: "ri:book-line", color: "text-amber-600" },
		Speech: { icon: "ri:mic-line", color: "text-pink-600" },
		Introspection: { icon: "ri:lightbulb-line", color: "text-indigo-600" },
		Entities: { icon: "ri:user-line", color: "text-slate-600" },
		Unknown: { icon: "ri:database-2-line", color: "text-neutral-600" },
	};

	function getDomainIcon(domain: string): string {
		return domainConfig[domain]?.icon || domainConfig.Unknown.icon;
	}

	function getDomainColor(domain: string): string {
		return domainConfig[domain]?.color || domainConfig.Unknown.color;
	}

	function formatCount(count: number): string {
		if (count === 0) return "No records";
		if (count === 1) return "1 record";
		if (count < 1000) return `${count} records`;
		if (count < 1000000) return `${(count / 1000).toFixed(1)}k records`;
		return `${(count / 1000000).toFixed(1)}M records`;
	}

	// Group ontologies by domain
	let groupedOntologies: Record<string, typeof data.ontologies> = {};
	$: {
		// Initialize all domains from domainConfig (except Unknown)
		const allDomains = Object.keys(domainConfig).filter(d => d !== 'Unknown');
		groupedOntologies = allDomains.reduce((acc, domain) => {
			acc[domain] = [];
			return acc;
		}, {} as Record<string, typeof data.ontologies>);

		// Populate with actual ontologies
		data.ontologies.forEach((ontology) => {
			if (!groupedOntologies[ontology.domain]) {
				groupedOntologies[ontology.domain] = [];
			}
			groupedOntologies[ontology.domain].push(ontology);
		});
	}

	// Track which sample records are expanded
	let expandedSamples: Set<string> = new Set();

	function toggleSample(ontologyName: string) {
		if (expandedSamples.has(ontologyName)) {
			expandedSamples.delete(ontologyName);
		} else {
			expandedSamples.add(ontologyName);
		}
		expandedSamples = expandedSamples; // Trigger reactivity
	}
</script>

<Page>
	<div class="max-w-7xl">
		<!-- Header -->
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">
				Ontologies
			</h1>
			<p class="text-neutral-600">
				Normalized data streams and signals for cross-source analysis,
				identifying gaps in personal data that fuel personal AI.
			</p>
			{#if data.ontologies.length > 0}
				<p class="text-sm text-neutral-500 mt-2">
					{data.ontologies.length} ontology {data.ontologies
						.length === 1
						? "table"
						: "tables"} available
				</p>
			{/if}
		</div>

		{#if data.ontologies.length === 0}
			<!-- Empty State -->
			<div
				class="border border-neutral-200 rounded-lg p-12 text-center bg-neutral-50"
			>
				<iconify-icon
					icon="ri:database-2-line"
					class="text-6xl text-neutral-300 mb-4"
				></iconify-icon>
				<h3 class="text-lg font-medium text-neutral-900 mb-2">
					No ontologies available
				</h3>
				<p class="text-neutral-600">
					Connect and sync data sources to populate your ontologies
				</p>
			</div>
		{:else}
			<!-- Ontologies grouped by domain -->
			{#each Object.entries(groupedOntologies) as [domain, ontologies]}
				<div class="mb-8">
					<!-- Domain Header -->
					<div class="flex items-center gap-3 mb-4">
						<iconify-icon
							icon={getDomainIcon(domain)}
							class="text-2xl {getDomainColor(domain)}"
						></iconify-icon>
						<h2
							class="text-xl font-serif font-medium text-neutral-900"
						>
							{domain}
							<span
								class="text-neutral-400 text-sm font-normal ml-2"
								>({ontologies.length})</span
							>
						</h2>
					</div>

					<!-- Ontologies Cards or Empty State -->
					{#if ontologies.length === 0}
						<div
							class="border border-dashed border-neutral-300 rounded-lg p-8 text-center bg-neutral-50/50"
						>
							<p class="text-sm text-neutral-500">
								Nothing here yet
							</p>
						</div>
					{:else}
						<div
							class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
						>
							{#each ontologies as ontology}
								<div
									class="p-4 bg-white border border-neutral-200 rounded-lg hover:border-neutral-300 transition-all duration-200"
								>
									<!-- Header -->
									<div class="mb-3">
										<h3
											class="font-medium text-neutral-900 text-sm mb-1"
										>
											{ontology.name}
										</h3>
										<p class="text-xs text-neutral-500">
											{formatCount(ontology.record_count)}
										</p>
									</div>

									<!-- Sample Record Section -->
									{#if ontology.sample_record}
										<div
											class="mt-3 pt-3 border-t border-neutral-100"
										>
											<button
												on:click={() =>
													toggleSample(ontology.name)}
												class="w-full text-left flex items-center justify-between text-xs text-neutral-600 hover:text-neutral-900 transition-colors"
											>
												<span>Sample record</span>
												<iconify-icon
													icon={expandedSamples.has(
														ontology.name,
													)
														? "ri:arrow-up-s-line"
														: "ri:arrow-down-s-line"}
													class="text-base"
												></iconify-icon>
											</button>

											{#if expandedSamples.has(ontology.name)}
												<div
													class="mt-2 p-2 bg-neutral-50 rounded border border-neutral-200 max-h-48 overflow-auto"
												>
													<pre
														class="text-xs text-neutral-700 whitespace-pre-wrap break-words font-mono">{JSON.stringify(
															ontology.sample_record,
															null,
															2,
														)}</pre>
												</div>
											{/if}
										</div>
									{:else if ontology.record_count === 0}
										<div
											class="mt-3 pt-3 border-t border-neutral-100"
										>
											<p
												class="text-xs text-neutral-400 italic"
											>
												No data yet
											</p>
										</div>
									{/if}
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</Page>