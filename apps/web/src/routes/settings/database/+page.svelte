<script lang="ts">
	import { onMount } from "svelte";
	import { Page, Button, Spinner } from "$lib/components";
	import { toast } from "svelte-sonner";
	import "iconify-icon";

	interface DatabaseUser {
		id: string;
		name: string;
		permissionLevel: "readonly" | "readwrite" | "full";
		connectionString?: string;
		createdAt: string;
		lastUsed: string | null;
		revokedAt: string | null;
	}

	let databaseUsers = $state<DatabaseUser[]>([]);
	let loading = $state(false);
	let creatingUser = $state(false);
	let showConnectionString = $state<{ [key: string]: boolean }>({});
	let newConnectionString = $state<string | null>(null);
	let confirmAcknowledge = $state("");
	let selectedPermission = $state<"readonly" | "readwrite" | "full" | null>(
		null,
	);
	let showRevoked = $state(false);

	const permissionDescriptions = {
		readonly: {
			title: "Read-Only",
			description: "View and analyze data",
			icon: "solar:eye-bold-duotone",
			color: "blue",
			grants: ["SELECT"],
		},
		readwrite: {
			title: "Read & Write",
			description: "Modify existing data",
			icon: "solar:pen-bold-duotone",
			color: "indigo",
			grants: ["SELECT", "INSERT", "UPDATE"],
		},
		full: {
			title: "Full Access",
			description: "Complete control including deletions",
			icon: "solar:shield-warning-bold-duotone",
			color: "rose",
			grants: ["SELECT", "INSERT", "UPDATE", "DELETE"],
			warning: true,
		},
	};

	onMount(async () => {
		await loadDatabaseUsers();
	});

	// Reload users when showRevoked changes
	$effect(() => {
		showRevoked;
		loadDatabaseUsers();
	});

	async function loadDatabaseUsers() {
		loading = true;
		try {
			const response = await fetch(
				`/api/database/users${showRevoked ? "?includeRevoked=true" : ""}`,
			);
			if (response.ok) {
				databaseUsers = await response.json();
			}
		} catch (error) {
			console.error("Failed to load database users:", error);
			toast.error("Failed to load database users");
		} finally {
			loading = false;
		}
	}

	async function createDatabaseUser(
		level: "readonly" | "readwrite" | "full",
	) {
		if (
			level === "full" &&
			confirmAcknowledge.toLowerCase() !== "i acknowledge"
		) {
			toast.error('Please type "I acknowledge" to confirm full access');
			return;
		}

		creatingUser = true;
		try {
			const response = await fetch("/api/database/users", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ permissionLevel: level }),
			});

			if (response.ok) {
				const data = await response.json();
				newConnectionString = data.connectionString;
				await loadDatabaseUsers();
				selectedPermission = null;
				confirmAcknowledge = "";
				toast.success("Database user created successfully");
			} else {
				const error = await response.json();
				toast.error(`Failed to create user: ${error.message}`);
			}
		} catch (error) {
			console.error("Failed to create database user:", error);
			toast.error("Failed to create database user");
		} finally {
			creatingUser = false;
		}
	}

	async function revokeUser(userId: string, userName: string) {
		if (
			!confirm(
				`Are you sure you want to revoke access for ${userName}? This action cannot be undone.`,
			)
		) {
			return;
		}

		try {
			const response = await fetch(`/api/database/users/${userId}`, {
				method: "DELETE",
			});

			if (response.ok) {
				await loadDatabaseUsers();
				toast.success("User access revoked");
			} else {
				toast.error("Failed to revoke user");
			}
		} catch (error) {
			console.error("Failed to revoke user:", error);
			toast.error("Failed to revoke user");
		}
	}

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text);
		toast.success("Copied to clipboard");
	}

	function formatDate(dateString: string) {
		return new Date(dateString).toLocaleDateString("en-US", {
			month: "short",
			day: "numeric",
			year: "numeric",
			hour: "2-digit",
			minute: "2-digit",
		});
	}
</script>

<Page>
	<!-- Header Section -->
	<div class="mb-8">
		<h1 class="text-3xl text-neutral-900 font-mono mb-2">
			Database Access
		</h1>
		<p class="text-neutral-600">
			Create secure PostgreSQL credentials for direct database connections
		</p>
	</div>

	<!-- New Connection String Alert -->
	{#if newConnectionString}
		<div
			class="mb-8 p-6 bg-gradient-to-br from-green-50 to-emerald-50 border border-green-200 rounded-xl"
		>
			<div class="flex items-start gap-3">
				<div class="flex-shrink-0 mt-1">
					<iconify-icon
						icon="solar:shield-check-bold-duotone"
						class="text-green-600"
						width="24"
					></iconify-icon>
				</div>
				<div class="flex-1">
					<h3 class="font-mono text-green-900 mb-1">
						New Database User Created
					</h3>
					<p class="text-sm text-green-700 mb-3">
						Save this connection string - it won't be shown again
					</p>
					<div
						class="bg-white/80 backdrop-blur rounded-lg p-4 font-mono text-xs break-all border border-green-200"
					>
						{newConnectionString}
					</div>
					<div class="mt-4 flex gap-2">
						<Button
							text="Copy Connection String"
							variant="filled"
							onclick={() =>
								copyToClipboard(newConnectionString || "")}
						/>
						<Button
							text="Done"
							variant="outline"
							onclick={() => (newConnectionString = null)}
						/>
					</div>
				</div>
			</div>
		</div>
	{/if}

	<!-- Create New Access -->
	<div class="mb-8">
		<h2 class="text-lg font-mono text-neutral-900 mb-4">
			Create New Access
		</h2>

		<div class="">
			{#if !selectedPermission}
				<div class="space-y-3">
					<p class="text-sm text-neutral-600 mb-4">
						Select the permission level for your new database user:
					</p>
					{#each Object.entries(permissionDescriptions) as [level, info]}
						<button
							onclick={() =>
								(selectedPermission = level as
									| "readonly"
									| "readwrite"
									| "full")}
							class="cursor-pointer w-full p-4 bg-neutral-50 hover:bg-neutral-100 border border-neutral-200 rounded-lg transition-colors text-left flex items-center gap-4"
						>
							<iconify-icon
								icon={info.icon}
								class={level === "readonly"
									? "text-blue-500"
									: level === "readwrite"
										? "text-indigo-500"
										: "text-rose-500"}
								width="24"
							></iconify-icon>
							<div class="flex-1">
								<div class="flex items-center gap-2">
									<span class="font-mono text-neutral-900"
										>{info.title}</span
									>
									{#if "warning" in info && info.warning}
										<span
											class="text-xs px-2 py-0.5 bg-rose-100 text-rose-700 rounded-full"
										>
											Caution
										</span>
									{/if}
								</div>
								<p class="text-sm text-neutral-500 mt-1">
									{info.description} ·
									{#each info.grants as grant, i}
										<span class="font-mono text-xs"
											>{grant}</span
										>{#if i < info.grants.length - 1},
										{/if}
									{/each}
								</p>
							</div>
							<iconify-icon
								icon="solar:alt-arrow-right-linear"
								class="text-neutral-400"
								width="20"
							></iconify-icon>
						</button>
					{/each}
				</div>
			{:else}
				<div class="space-y-4">
					<div class="flex items-center gap-3">
						<iconify-icon
							icon={permissionDescriptions[selectedPermission]
								.icon}
							class={selectedPermission === "readonly"
								? "text-blue-500"
								: selectedPermission === "readwrite"
									? "text-indigo-500"
									: "text-rose-500"}
							width="24"
						></iconify-icon>
						<h3 class="font-mono text-neutral-900">
							Create {permissionDescriptions[selectedPermission]
								.title} User
						</h3>
					</div>

					{#if selectedPermission === "full"}
						<div
							class="p-4 bg-rose-50 border border-rose-200 rounded-lg"
						>
							<p class="text-sm text-rose-800 mb-3">
								⚠️ Full access includes DELETE permissions. Type <strong
									>"I acknowledge"</strong
								> to confirm you understand the risks.
							</p>
							<input
								type="text"
								bind:value={confirmAcknowledge}
								placeholder="Type 'I acknowledge' to confirm"
								class="w-full px-3 py-2 border border-rose-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-rose-500"
							/>
						</div>
					{/if}

					<div class="flex gap-2 pt-2">
						<Button
							text={creatingUser ? "Creating..." : "Create User"}
							variant="filled"
							disabled={creatingUser ||
								(selectedPermission === "full" &&
									confirmAcknowledge.toLowerCase() !==
										"i acknowledge")}
							onclick={() =>
								createDatabaseUser(selectedPermission)}
						/>
						<Button
							text="Cancel"
							variant="outline"
							onclick={() => {
								selectedPermission = null;
								confirmAcknowledge = "";
							}}
						/>
					</div>
				</div>
			{/if}
		</div>
	</div>

	<!-- Existing Users -->
	<div class="mb-8">
		<div class="flex items-center justify-between mb-4">
			<h2 class="text-lg font-mono text-neutral-900">
				Active Credentials
			</h2>
			<div class="flex items-center gap-4">
				{#if databaseUsers.length > 0}
					<span class="text-sm text-neutral-500">
						{databaseUsers.filter((u) => !u.revokedAt).length} active
						{#if databaseUsers.filter((u) => u.revokedAt).length > 0}
							· {databaseUsers.filter((u) => u.revokedAt).length} revoked
						{/if}
					</span>
				{/if}
				<button
					onclick={() => (showRevoked = !showRevoked)}
					class="text-sm cursor-pointer text-neutral-600 hover:text-neutral-900 font-mono transition-colors flex items-center gap-1"
				>
					{showRevoked ? "Hide" : "Show"} revoked
					<iconify-icon
						icon={showRevoked
							? "solar:eye-closed-linear"
							: "solar:eye-linear"}
						width="16"
					></iconify-icon>
				</button>
			</div>
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-12">
				<Spinner />
			</div>
		{:else if databaseUsers.length === 0}
			<div
				class="text-center py-12 bg-neutral-50 rounded-xl border-2 border-dashed border-neutral-300"
			>
				<iconify-icon
					icon="solar:database-bold-duotone"
					class="text-neutral-400 mb-3"
					width="48"
				></iconify-icon>
				<p class="text-neutral-600">No database users created yet</p>
				<p class="text-sm text-neutral-500 mt-1">
					Create your first user above to get started
				</p>
			</div>
		{:else}
			<div class="grid gap-3">
				{#each databaseUsers as user}
					<div
						class="p-4 bg-white border border-neutral-200 rounded-lg transition-all {user.revokedAt
							? 'opacity-60'
							: 'hover:shadow-md'}"
					>
						<div class="flex items-start justify-between">
							<div class="flex-1">
								<div class="flex items-center gap-3 mb-2">
									<span
										class="font-mono {user.revokedAt
											? 'text-neutral-500 line-through'
											: 'text-neutral-900'}"
										>{user.name}</span
									>
									{#if user.revokedAt}
										<span
											class="px-2 py-0.5 text-xs rounded-full font-medium bg-neutral-100 text-neutral-600"
										>
											Revoked
										</span>
									{:else}
										<span
											class="px-2 py-0.5 text-xs rounded-full font-medium
											{user.permissionLevel === 'readonly'
												? 'bg-blue-100 text-blue-700'
												: user.permissionLevel ===
													  'readwrite'
													? 'bg-indigo-100 text-indigo-700'
													: 'bg-rose-100 text-rose-700'}"
										>
											{permissionDescriptions[
												user.permissionLevel
											].title}
										</span>
									{/if}
								</div>
								<div
									class="flex items-center gap-4 text-sm text-neutral-500"
								>
									{#if user.revokedAt}
										<span class="text-rose-600">
											Revoked {formatDate(user.revokedAt)}
										</span>
									{:else}
										<span>
											Created {formatDate(user.createdAt)}
										</span>
										{#if user.lastUsed}
											<span>
												• Last used {formatDate(
													user.lastUsed,
												)}
											</span>
										{:else}
											<span>• Never used</span>
										{/if}
									{/if}
								</div>
							</div>
							{#if !user.revokedAt}
								<Button
									text="Revoke"
									variant="danger"
									onclick={() =>
										revokeUser(user.id, user.name)}
								/>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Connection Examples -->
	<div class="mb-8">
		<h2 class="text-lg font-mono text-neutral-900 mb-4">
			Connection Examples
		</h2>

		<div class="grid md:grid-cols-2 gap-4">
			<div class="p-4 bg-white border border-neutral-200 rounded-lg">
				<h3 class="font-mono text-sm mb-2 flex items-center gap-2">
					<iconify-icon icon="logos:python" width="20"></iconify-icon>
					Python
				</h3>
				<pre
					class="bg-neutral-900 text-neutral-100 p-3 rounded text-xs font-mono overflow-x-auto">
import psycopg2
import pandas as pd

conn = psycopg2.connect("YOUR_CONNECTION_STRING")
df = pd.read_sql("SELECT * FROM signals LIMIT 10", conn)
print(df.head())</pre>
			</div>

			<div class="p-4 bg-white border border-neutral-200 rounded-lg">
				<h3 class="font-mono text-sm mb-2 flex items-center gap-2">
					<iconify-icon icon="logos:javascript" width="20"
					></iconify-icon>
					JavaScript
				</h3>
				<pre
					class="bg-neutral-900 text-neutral-100 p-3 rounded text-xs font-mono overflow-x-auto">
import {`{ Client }`} from 'pg';

const client = new Client({`{ 
  connectionString: 'YOUR_CONNECTION_STRING' 
}`});
await client.connect();
const result = await client.query('SELECT * FROM signals LIMIT 10');</pre>
			</div>

			<div
				class="p-4 bg-white border border-neutral-200 rounded-lg md:col-span-2"
			>
				<h3 class="font-mono text-sm mb-2">Compatible Tools</h3>
				<p class="text-sm text-neutral-600">
					Use your connection string with any PostgreSQL client:
					TablePlus, DBeaver, pgAdmin, DataGrip, psql, and more.
				</p>
			</div>
		</div>
	</div>
</Page>
