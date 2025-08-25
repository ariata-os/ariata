<script lang="ts">
	import { Page, Badge } from "$lib/components";
	import {
		formatDate,
		formatRelativeTime,
		getStatusBadgeVariant,
	} from "$lib/utils/format";
	import type { PageData } from "./$types";
	import "iconify-icon";

	let { data }: { data: PageData } = $props();
	const { source } = data;

	// Check if OAuth is expired
	function isOAuthExpired(expiresAt: Date | string | null): boolean {
		if (!expiresAt) return false;
		const expiry =
			typeof expiresAt === "string" ? new Date(expiresAt) : expiresAt;
		return expiry < new Date();
	}

	// Get status display text
	function getStatusText(status: string): string {
		switch (status) {
			case 'authenticated':
				return 'Setup Required';
			case 'active':
				return 'Active';
			case 'paused':
				return 'Paused';
			case 'needs_reauth':
				return 'Reconnect Required';
			case 'error':
				return 'Error';
			default:
				return status;
		}
	}

	// Get status badge variant
	function getSourceStatusVariant(status: string): "success" | "error" | "warning" | "default" | "info" {
		switch (status) {
			case 'active':
				return 'success';
			case 'authenticated':
				return 'warning';
			case 'paused':
				return 'default';
			case 'needs_reauth':
			case 'error':
				return 'error';
			default:
				return 'default';
		}
	}
</script>

<Page>
	<div class="space-y-6">
		<!-- Header -->
		<div class="flex items-start justify-between">
			<div>
				<div class="flex items-center gap-3 mb-2">
					{#if source.icon}
						<iconify-icon
							icon={source.icon}
							width="32"
							height="32"
							class="text-neutral-700 p-4 bg-neutral-100 border border-neutral-200 rounded-lg"
						></iconify-icon>
					{/if}
					<div class="flex flex-col gap-1">
						<h1 class="text-3xl font-mono text-neutral-900">
							{source.instanceName}
						</h1>
						<p class="text-neutral-600">
							{source.description}
						</p>
					</div>
				</div>
			</div>
			<Badge variant={getSourceStatusVariant(source.status)} size="md">
				{getStatusText(source.status)}
			</Badge>
		</div>

		<!-- Basic Information -->
		<div
			class="bg-white rounded-lg border border-neutral-200 overflow-hidden"
		>
			<div class="px-6 py-4 bg-neutral-100 border-b border-neutral-200">
				<h2 class="text-lg font-mono font-medium text-neutral-900">
					Basic Information
				</h2>
			</div>
			<div class="p-6">
				<dl class="grid grid-cols-1 gap-4 sm:grid-cols-2">
					<div>
						<dt class="text-sm font-medium text-neutral-500">
							Source Type
						</dt>
						<dd class="mt-1 text-sm text-neutral-900">
							{source.displayName}
						</dd>
					</div>
					<div>
						<dt class="text-sm font-medium text-neutral-500">
							Instance Name
						</dt>
						<dd class="mt-1 text-sm text-neutral-900">
							{source.instanceName}
						</dd>
					</div>
					<div>
						<dt class="text-sm font-medium text-neutral-500">
							Platform
						</dt>
						<dd class="mt-1 text-sm text-neutral-900">
							{source.platform || "—"}
						</dd>
					</div>
					<div>
						<dt class="text-sm font-medium text-neutral-500">
							Authentication Type
						</dt>
						<dd class="mt-1 text-sm text-neutral-900">
							{source.authType || "—"}
						</dd>
					</div>
					<div>
						<dt class="text-sm font-medium text-neutral-500">
							Source ID
						</dt>
						<dd class="mt-1 text-sm text-neutral-900 font-mono">
							{source.id}
						</dd>
					</div>
					<div>
						<dt class="text-sm font-medium text-neutral-500">
							Created
						</dt>
						<dd class="mt-1 text-sm text-neutral-900">
							{formatDate(source.createdAt)}
						</dd>
					</div>
				</dl>
			</div>
		</div>

		<!-- Sync Information -->
		<div
			class="bg-white rounded-lg border border-neutral-200 overflow-hidden"
		>
			<div class="px-6 py-4 bg-neutral-100 border-b border-neutral-200">
				<h2 class="text-lg font-mono font-medium text-neutral-900">
					Sync Information
				</h2>
			</div>
			<div class="p-6">
				<dl class="grid grid-cols-1 gap-4 sm:grid-cols-2">
					<div>
						<dt class="text-sm font-medium text-neutral-500">
							Last Sync
						</dt>
						<dd class="mt-1 text-sm text-neutral-900">
							{formatRelativeTime(source.lastSyncAt)}
						</dd>
					</div>
					<div>
						<dt class="text-sm font-medium text-neutral-500">
							Sync Status
						</dt>
						<dd class="mt-1">
							{#if source.lastSyncStatus}
								<Badge
									variant={getStatusBadgeVariant(
										source.lastSyncStatus,
									)}
									size="sm"
								>
									{source.lastSyncStatus}
								</Badge>
							{:else}
								<span class="text-sm text-neutral-500"
									>Never synced</span
								>
							{/if}
						</dd>
					</div>
					{#if source.lastSyncError}
						<div class="sm:col-span-2">
							<dt class="text-sm font-medium text-neutral-500">
								Last Error
							</dt>
							<dd class="mt-1 text-sm text-red-600 font-mono">
								{source.lastSyncError}
							</dd>
						</div>
					{/if}
					<div>
						<dt class="text-sm font-medium text-neutral-500">
							Last Updated
						</dt>
						<dd class="mt-1 text-sm text-neutral-900">
							{formatDate(source.updatedAt)}
						</dd>
					</div>
				</dl>
			</div>
		</div>

		<!-- Device Information (if applicable) -->
		{#if source.deviceId || source.deviceType || source.pairedDeviceName}
			<div
				class="bg-white rounded-lg border border-neutral-200 overflow-hidden"
			>
				<div
					class="px-6 py-4 bg-neutral-100 border-b border-neutral-200"
				>
					<h2 class="text-lg font-mono font-medium text-neutral-900">
						Device Information
					</h2>
				</div>
				<div class="p-6">
					<dl class="grid grid-cols-1 gap-4 sm:grid-cols-2">
						{#if source.deviceId}
							<div>
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									Device ID
								</dt>
								<dd
									class="mt-1 text-sm text-neutral-900 font-mono"
								>
									{source.deviceId}
								</dd>
							</div>
						{/if}
						{#if source.deviceType}
							<div>
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									Device Type
								</dt>
								<dd class="mt-1 text-sm text-neutral-900">
									{source.deviceType}
								</dd>
							</div>
						{/if}
						{#if source.pairedDeviceName}
							<div>
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									Device Name
								</dt>
								<dd class="mt-1 text-sm text-neutral-900">
									{source.pairedDeviceName}
								</dd>
							</div>
						{/if}
						{#if source.deviceLastSeen}
							<div>
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									Last Seen
								</dt>
								<dd class="mt-1 text-sm text-neutral-900">
									{formatRelativeTime(source.deviceLastSeen)}
								</dd>
							</div>
						{/if}
						{#if source.deviceToken}
							<div>
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									Device Token
								</dt>
								<dd
									class="mt-1 text-sm text-neutral-900 font-mono"
								>
									{source.deviceToken}
								</dd>
							</div>
						{/if}
					</dl>
				</div>
			</div>
		{/if}

		<!-- OAuth Information (if applicable) -->
		{#if source.oauthAccessToken || source.oauthExpiresAt || (source.scopes && source.scopes.length > 0)}
			<div
				class="bg-white rounded-lg border border-neutral-200 overflow-hidden"
			>
				<div
					class="px-6 py-4 bg-neutral-100 border-b border-neutral-200"
				>
					<h2 class="text-lg font-mono font-medium text-neutral-900">
						OAuth Information
					</h2>
				</div>
				<div class="p-6">
					<dl class="grid grid-cols-1 gap-4 sm:grid-cols-2">
						{#if source.oauthAccessToken}
							<div>
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									Access Token
								</dt>
								<dd
									class="mt-1 text-sm text-neutral-900 font-mono"
								>
									{source.oauthAccessToken}
								</dd>
							</div>
						{/if}
						{#if source.oauthRefreshToken}
							<div>
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									Refresh Token
								</dt>
								<dd
									class="mt-1 text-sm text-neutral-900 font-mono"
								>
									{source.oauthRefreshToken}
								</dd>
							</div>
						{/if}
						{#if source.oauthExpiresAt}
							<div>
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									Token Expires
								</dt>
								<dd class="mt-1">
									<span class="text-sm text-neutral-900">
										{formatDate(source.oauthExpiresAt)}
									</span>
									{#if isOAuthExpired(source.oauthExpiresAt)}
										<Badge
											variant="error"
											size="sm"
											class="ml-2"
										>
											Expired
										</Badge>
									{/if}
								</dd>
							</div>
						{/if}
						{#if source.scopes && source.scopes.length > 0}
							<div class="sm:col-span-2">
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									Scopes
								</dt>
								<dd class="mt-1 flex flex-wrap gap-2">
									{#each source.scopes as scope}
										<span
											class="inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-neutral-100 text-neutral-700"
										>
											{scope}
										</span>
									{/each}
								</dd>
							</div>
						{/if}
					</dl>
				</div>
			</div>
		{/if}

		<!-- Additional Metadata (if applicable) -->
		{#if source.sourceMetadata && Object.keys(source.sourceMetadata).length > 0}
			<div
				class="bg-white rounded-lg border border-neutral-200 overflow-hidden"
			>
				<div
					class="px-6 py-4 bg-neutral-100 border-b border-neutral-200"
				>
					<h2 class="text-lg font-mono font-medium text-neutral-900">
						Additional Metadata
					</h2>
				</div>
				<div class="p-6">
					<dl class="grid grid-cols-1 gap-4">
						{#each Object.entries(source.sourceMetadata) as [key, value]}
							<div>
								<dt
									class="text-sm font-medium text-neutral-500"
								>
									{key}
								</dt>
								<dd
									class="mt-1 text-sm text-neutral-900 font-mono"
								>
									{typeof value === "object"
										? JSON.stringify(value, null, 2)
										: value}
								</dd>
							</div>
						{/each}
					</dl>
				</div>
			</div>
		{/if}
	</div>
</Page>
