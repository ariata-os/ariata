<script lang="ts">
	import { Page } from "$lib";
	import "iconify-icon";
</script>

<Page>
	<div class="max-w-3xl">
		<!-- Header -->
		<div class="mb-8">
			<h1 class="text-3xl font-serif font-medium text-neutral-900 mb-2">
				Account
			</h1>
			<p class="text-neutral-600">
				Manage your account settings and preferences
			</p>
		</div>

		<!-- Account Settings Content -->
		<div class="space-y-6">
			<!-- Profile Section -->
			<div class="bg-white border border-neutral-200 rounded-lg p-6">
				<h2 class="text-lg font-medium text-neutral-900 mb-4">
					Profile
				</h2>
				<div class="space-y-4">
					<div>
						<label
							for="email"
							class="block text-sm font-medium text-neutral-700 mb-2"
						>
							Email
						</label>
						<input
							type="email"
							id="email"
							class="w-full px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
							placeholder="your@email.com"
							disabled
						/>
						<p class="text-xs text-neutral-500 mt-1">
							Email settings coming soon
						</p>
					</div>
				</div>
			</div>

			<!-- Preferences Section -->
			<div class="bg-white border border-neutral-200 rounded-lg p-6">
				<h2 class="text-lg font-medium text-neutral-900 mb-4">
					Preferences
				</h2>
				<div class="space-y-4">
					<div class="flex items-center justify-between">
						<div>
							<p class="text-sm font-medium text-neutral-900">
								Theme
							</p>
							<p class="text-xs text-neutral-500">
								Customize your interface appearance
							</p>
						</div>
						<select
							class="px-3 py-2 border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-neutral-900 focus:border-transparent"
							disabled
						>
							<option>Light</option>
							<option>Dark</option>
							<option>System</option>
						</select>
					</div>
					<p class="text-xs text-neutral-500">
						Theme preferences coming soon
					</p>
				</div>
			</div>

			<!-- About Section -->
			<div class="bg-white border border-neutral-200 rounded-lg p-6">
				<h2 class="text-lg font-medium text-neutral-900 mb-4">
					About
				</h2>
				<div class="space-y-2 text-sm text-neutral-600">
					<div class="flex items-center justify-between">
						<span>Version</span>
						<span class="font-medium text-neutral-900">1.0.0</span>
					</div>
				</div>
			</div>
		</div>
	</div>
</Page>
