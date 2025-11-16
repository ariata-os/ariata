<script lang="ts">
	import { Page } from '$lib';
	import { goto } from '$app/navigation';
</script>

<Page>
	<div class="max-w-4xl">
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">Axiology</h1>
			<p class="text-neutral-600 mb-1">"What you do reveals what you value"</p>
			<p class="text-neutral-600">
				Track your values across different time horizons, from daily actions to lifetime purpose.
			</p>
		</div>

		<div class="space-y-4">
			<!-- Tasks -->
			<button
				type="button"
				onclick={() => goto('/axiology/tasks')}
				class="w-full text-left border border-neutral-200 rounded-lg p-6 hover:border-neutral-300 transition-colors"
			>
				<h2 class="text-xl font-serif font-medium text-neutral-900 mb-2">Tasks</h2>
				<p class="text-neutral-600 mb-2">Daily and weekly completable items</p>
				<p class="text-sm text-neutral-500">
					Examples: Read more books, Exercise 3x/week, Write daily journal
				</p>
			</button>

			<!-- Initiatives -->
			<div class="opacity-50 border border-neutral-200 rounded-lg p-6">
				<h2 class="text-xl font-serif font-medium text-neutral-900 mb-2">
					Initiatives
					<span class="text-sm font-normal text-neutral-500">Coming Soon</span>
				</h2>
				<p class="text-neutral-600 mb-2">Medium-term commitments (month-quarter)</p>
				<p class="text-sm text-neutral-500">
					Examples: Launch side project, Learn Spanish basics, Build fitness habit
				</p>
			</div>

			<!-- Aspirations -->
			<div class="opacity-50 border border-neutral-200 rounded-lg p-6">
				<h2 class="text-xl font-serif font-medium text-neutral-900 mb-2">
					Aspirations
					<span class="text-sm font-normal text-neutral-500">Coming Soon</span>
				</h2>
				<p class="text-neutral-600 mb-2">Long-term life goals (multi-year bucket list)</p>
				<p class="text-sm text-neutral-500">
					Examples: Run a marathon, Visit all 7 continents, Master piano
				</p>
			</div>

			<!-- Telos -->
			<div class="opacity-50 border border-neutral-200 rounded-lg p-6">
				<h2 class="text-xl font-serif font-medium text-neutral-900 mb-2">
					Telos
					<span class="text-sm font-normal text-neutral-500">Coming Soon</span>
				</h2>
				<p class="text-neutral-600 mb-2">Ultimate life purpose (who you're becoming)</p>
				<p class="text-sm text-neutral-500">Singular, evolving statement of your direction</p>
			</div>
		</div>

		<!-- Philosophy -->
		<div class="mt-8 border border-neutral-200 rounded-lg p-6">
			<h3 class="text-lg font-serif font-medium text-neutral-900 mb-3">
				The Hierarchy of Values
			</h3>
			<p class="text-neutral-600 mb-3">
				Axiology organizes your pursuits across different time horizons. This helps you see
				how your day-to-day activities align with your deepest values.
			</p>
			<p class="text-sm text-neutral-500">
				Tasks (days-weeks) → Initiatives (months-quarters) → Aspirations (years-decades) →
				Telos (lifetime)
			</p>
		</div>
	</div>
</Page>
