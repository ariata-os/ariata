<!--
  Devices — the unified "+ paired things" list. Every browser tab, mobile app,
  Mac collector, and sensor that's paired with this box shows up here. The
  user can revoke any of them; the box invalidates the credential AND evicts
  the WG peer in a single transaction. There is no "log out of just one tab"
  here — that's signout. This is "this hardware/app no longer has any
  authority."

  The server is ROW ONE of the same table, not a card above it. Until 2026-09-08
  it sat in its own box over the grid ("This server — the release it runs"),
  which made the one machine everything else is paired to the one machine you
  could not search, sort or read the version of alongside the rest. It is still
  not a peer — it cannot be revoked, and its lifecycle is an upgrade — so its
  row carries an Updates action where the others carry Revoke, and it is pinned
  first in the default order.
-->
<script lang="ts">
	import type { Tab } from "$lib/tabs/types";
	import { Page, Button, Badge, LoadingState, ErrorState } from "$lib";
	import Icon from "$lib/components/Icon.svelte";
	import UniversalDataGrid, {
		type Column,
	} from "$lib/components/datagrid/UniversalDataGrid.svelte";
	import {
		listDevices,
		pairMint,
		pairDeny,
		pairStatus as pairStatusApi,
		getUpdateStatus,
		getSystemTelemetry,
		type UpdateStatus,
	} from "$lib/api/client";
	import { createResource } from "$lib/utils/resource.svelte";
	import { formatDateTime } from "$lib/utils/dateUtils";
	import { toast } from "svelte-sonner";
	import { isTauri } from "$lib/utils/platform";
	import { windowShellStore } from "$lib/stores/window-shell.svelte";
	import ReopenSection from "$lib/components/settings/ReopenSection.svelte";
	import {
		kindLabel,
		kindIcon,
		deniedPermissions,
		deviceHref,
		revokeDeviceFlow,
		type Device,
		type DevicesResponse,
	} from "$lib/devices/shared";

	let { tab, active }: { tab: Tab; active: boolean } = $props();

	const res = createResource(() => listDevices<DevicesResponse>());

	// The server's own facts, fetched beside the device list rather than as
	// part of it: the box does not list itself as a device (it has no
	// credential to itself), so its row is assembled here from the update
	// status — release, channel, whether a newer one exists — and the host
	// telemetry for the hostname and OS. Both are best-effort: the row exists
	// the moment the page opens, and fills in as each answer arrives. A failed
	// check shows a blank, never a stale or invented value.
	type HostInfo = { hostname?: string | null; os?: string | null; kernel?: string | null };
	let update = $state<UpdateStatus | null>(null);
	let host = $state<HostInfo | null>(null);

	async function loadServer() {
		const [u, t] = await Promise.allSettled([
			getUpdateStatus(),
			getSystemTelemetry<{ host?: HostInfo }>(),
		]);
		update = u.status === "fulfilled" ? u.value : null;
		host = t.status === "fulfilled" ? (t.value?.host ?? null) : null;
	}

	$effect(() => {
		loadServer();
	});

	async function reloadAll() {
		await Promise.all([res.reload(), loadServer()]);
	}

	// One row shape for the server and every device, so the grid searches,
	// sorts and groups them as a single population.
	type Row = {
		id: string;
		is_server: boolean;
		label: string;
		/** Second line under the name — the kind, in a person's words. */
		kind: string;
		version: string | null;
		/** Second line under the version — the OS for the server, the
		 *  interface bundle for an app that carries its own release. */
		version_sub: string | null;
		/** Why this row is out of date, or null when it is not (or we cannot
		 *  tell — the two are deliberately drawn the same, as no mark). */
		behind: string | null;
		last_seen_at: string | null;
		paired_at: string | null;
		device: Device | null;
	};

	// A collector is a desktop_app credential that declared a data source at
	// pairing (the app itself pairs sourceless). Its kind badge saying
	// "Desktop" is what made `phf-virtues.local` unreadable next to the app.
	function isCollectorRow(d: Device): boolean {
		return d.kind === "desktop_app" && !!d.source_id;
	}

	const serverRow = $derived.by((): Row => {
		const name = host?.hostname ? `Virtues Server · ${host.hostname}` : "Virtues Server";
		const os = host?.os ?? null;
		const kernel = host?.kernel ?? null;
		return {
			id: "server",
			is_server: true,
			label: name,
			kind: "Server",
			version: update?.running_version ?? null,
			version_sub: os && kernel ? `${os} · ${kernel}` : (os ?? kernel),
			behind:
				update?.update_available && update.latest
					? `${update.latest} is available — open to install`
					: update?.update_available
						? "A newer release is available — open to install"
						: null,
			// It answered this request.
			last_seen_at: new Date().toISOString(),
			paired_at: null,
			device: null,
		};
	});

	// The server first, then the device you are holding, then each machine's
	// collector folded directly under the app that installed it
	// (`installed_by`, stamped at pair-consume from the token's minter). The
	// box orders by last-seen, which is *usually* right and reliably isn't
	// when you have just opened the app on a second machine; and a flat
	// last-seen order is what made one Mac read as two unrelated rows.
	// Collectors whose installing app is gone (revoked, or paired before the
	// join existed) list standalone at the end.
	const rows = $derived.by((): Row[] => {
		const all = res.data?.devices ?? [];
		const parents = all
			.filter((d) => !d.installed_by)
			.sort((a, b) => Number(b.is_current) - Number(a.is_current));
		const ordered: Device[] = [];
		for (const p of parents) {
			ordered.push(p, ...all.filter((d) => d.installed_by === p.id));
		}
		ordered.push(
			...all.filter(
				(d) => d.installed_by && !parents.some((p) => p.id === d.installed_by),
			),
		);
		const byId = new Map(all.map((d) => [d.id, d]));
		return [serverRow, ...ordered.map((d) => deviceRow(d, byId))];
	});

	function deviceRow(d: Device, byId: Map<string, Device>): Row {
		const collector = isCollectorRow(d);
		// The collector runs a build the app installs, so a lasting
		// disagreement means a relaunch has not happened — not that the
		// collector is wrong. Same rule the device page applies.
		const parent = d.installed_by ? byId.get(d.installed_by) : undefined;
		const behind =
			collector && d.version && parent?.app_version && d.version !== parent.app_version
				? `Behind the app that installed it (${parent.app_version}) — relaunch the app`
				: null;
		return {
			id: d.id,
			is_server: false,
			label: d.label,
			kind: collector ? "Collector" : kindLabel(d.kind),
			version: versionText(d),
			// An app that reports its own release also carries an interface
			// bundle; a collector's version IS its binary and has nothing under it.
			version_sub: d.app_version && d.version ? `interface ${d.version}` : (d.channel ?? null),
			behind,
			last_seen_at: d.last_seen_at,
			paired_at: d.paired_at,
			device: d,
		};
	}

	// The version a person means: the native app's own release when the device
	// reports one. The old single string was the UI-bundle identity — which,
	// for a paired desktop, is the box-served SPA and so mirrored the box:
	// "This device · 0.1.5-staging.65" on a Mac running app 1.0.22 was the
	// confusion that triggered the version audit. The full lattice (bundle,
	// sha, channel) still lives on the device detail page; here one honest
	// number beats three misleading ones. Bundle identity remains the headline
	// only for rows that have no app of their own (the console renders the
	// box's UI; a collector's `version` IS its binary once it reports).
	function versionText(device: Device): string | null {
		if (device.app_version) return device.app_version;
		if (!device.version) return null;
		const sha = device.sha && device.sha !== "dev" ? ` · ${device.sha}` : "";
		return `${device.version}${sha}`;
	}

	// "Connected" is a claim about now, and the box only knows when a device
	// last spoke to it. Five minutes covers every poll interval a paired app
	// runs at, so a device inside that window is talking to the box as you
	// read this; outside it, the honest thing is the timestamp itself, not a
	// coarser "3d ago". The server and the device in your hand are connected
	// by construction — one answered this request, the other made it.
	const CONNECTED_WINDOW_MS = 5 * 60 * 1000;
	function isConnected(row: Row): boolean {
		if (row.is_server || row.device?.is_current) return true;
		if (!row.last_seen_at) return false;
		return Date.now() - new Date(row.last_seen_at).getTime() < CONNECTED_WINDOW_MS;
	}

	function rowIcon(row: Row): string {
		return row.is_server ? "ri:server-line" : kindIcon(row.device!.kind);
	}

	function rowHref(row: Row): string {
		return row.is_server ? "/virtues/devices/server" : deviceHref(row.device!);
	}

	function openRow(row: Row) {
		windowShellStore.navigate(rowHref(row), { label: "Settings" });
	}

	// "+ Add device" modal state.
	let addOpen = $state(false);
	let mintLoading = $state(false);
	let mintError = $state<string | null>(null);
	let mintedToken = $state<string | null>(null);
	let mintedUrl = $state<string | null>(null);
	let mintedQrSvg = $state<string | null>(null);
	let mintedTokenId = $state<string | null>(null);
	let mintedExpiresAt = $state<string | null>(null);
	// No "pending": an authenticated mint is authorized on the spot, so the
	// confirm round-trip (and the /api/pair/confirm route it called, which no
	// longer exists) is gone.
	let pairStatus = $state<"authorized" | "consumed" | "expired" | "denied" | "idle">(
		"idle"
	);
	let consumedByLabel = $state<string | null>(null);
	let pollHandle: ReturnType<typeof setInterval> | null = null;

	async function revoke(device: Device) {
		if (await revokeDeviceFlow(device)) await res.reload();
	}

	function stopPolling() {
		if (pollHandle) {
			clearInterval(pollHandle);
			pollHandle = null;
		}
	}

	async function startAdd() {
		addOpen = true;
		mintError = null;
		mintedToken = null;
		mintedUrl = null;
		mintedTokenId = null;
		mintedExpiresAt = null;
		pairStatus = "idle";
		consumedByLabel = null;
		mintLoading = true;
		try {
			// No intended_kind. This minted "Add device" without one is redeemed
			// by whatever scans it — a phone, a Mac, the CLI — and the device
			// declares its own kind at consume. Naming one here meant naming
			// "browser", which the token's CHECK constraint rejects, so this
			// button failed on every click with `mint_failed`.
			const data = await pairMint();
			mintedToken = data.token;
			mintedUrl = data.pair_url;
			mintedQrSvg = data.qr_svg;
			mintedTokenId = data.id;
			mintedExpiresAt = data.expires_at;
			// Authenticated mints are minted `authorized` (see mint_pair_token) —
			// there is no confirm round-trip to wait through.
			pairStatus = "authorized";
			pollHandle = setInterval(pollStatus, 2000);
		} catch (e) {
			mintError = e instanceof Error ? e.message : "Could not mint pair token";
		} finally {
			mintLoading = false;
		}
	}

	async function denyPair() {
		if (!mintedTokenId) return;
		await pairDeny(mintedTokenId);
		pairStatus = "denied";
		stopPolling();
	}

	async function pollStatus() {
		if (!mintedTokenId) return;
		try {
			const data = await pairStatusApi(mintedTokenId);
			pairStatus = data.status as typeof pairStatus;
			if (data.consumed_by_label) {
				consumedByLabel = data.consumed_by_label;
			}
			if (
				data.status === "consumed" ||
				data.status === "expired" ||
				data.status === "denied"
			) {
				stopPolling();
				if (data.status === "consumed") {
					toast.success("New device paired");
					await res.reload();
				}
			}
		} catch {
			/* swallow */
		}
	}

	function closeAdd() {
		stopPolling();
		addOpen = false;
	}

	function copyUrl() {
		if (mintedUrl) {
			navigator.clipboard.writeText(mintedUrl);
			toast.success("URL copied");
		}
	}

	// Shown whenever we ARE a Mac app, not only on the `is_current` row.
	//
	// One Mac appears as TWO devices — the app ("Virtues Desktop", is_current)
	// and the collector (the .local hostname, is_current false) — and it is the
	// COLLECTOR that reports permissions. Gating on is_current therefore hid the
	// button behind a row that never carries a denial, so the fix shipped inert
	// (caught on live data 2026-08-13).
	//
	// The cost of relaxing it: someone with two Macs sees the button on the
	// other Mac's row too, where it opens the wrong machine's settings. Rare,
	// recoverable, and better than a control that cannot appear at all — but it
	// is why the label names this Mac rather than the device in the row.
	const canFix = $derived(isTauri);

	// Columns feed the grid's search/sort/group; the cells themselves come from
	// the tableRow snippet below. Date columns return the raw ISO string so sort
	// orders by time, not by the prose the cell displays.
	const columns: Column<Row>[] = [
		{
			key: "label",
			label: "Machine",
			icon: "ri:device-line",
			width: "38%",
			minWidth: "220px",
		},
		{
			// The row's second line already says the kind; keep the field for
			// grouping and search without spending a column on it.
			key: "kind",
			label: "Kind",
			hidden: true,
			groupable: true,
		},
		{
			key: "version",
			label: "Version",
			icon: "ri:git-commit-line",
			width: "24%",
			minWidth: "150px",
		},
		{
			key: "last_seen_at",
			label: "Last seen",
			icon: "ri:time-line",
			width: "20%",
			minWidth: "140px",
		},
		{
			key: "paired_at",
			label: "Paired",
			icon: "ri:link",
			width: "18%",
			minWidth: "120px",
			hideOnMobile: true,
		},
	];
</script>

<!--
	The page's own heading, not a hand-rolled one. This view used to draw its
	own `<h1 class="text-2xl font-semibold">` inside a bare `<Page>` with its own
	measure and padding — so Devices was a sans 24px heading in a 3xl column
	while every neighbouring room was a serif 30px heading in a 6xl one, for no
	reason anyone chose.
-->
<Page
	title="Devices"
	description="Your server, and every app, browser and sensor paired to it."
	maxWidth="wide"
>
	{#snippet actions()}
		<Button variant="primary" onclick={startAdd}>
			<Icon icon="ri:add-line" />
			Add device
		</Button>
	{/snippet}

	<!--
		The universal grid, not a hand-rolled list — Devices was the one list in
		the app with its own row markup, so it was also the one list without
		search, sort, grouping or a right-click menu. Row click is the drill-down
		(the grid stops propagation around rowActions and the fix button below,
		so the banner's "Open Full Disk Access" can't fire a navigation behind
		itself).
	-->
	<UniversalDataGrid
		items={rows}
		{columns}
		entityType="devices"
		loading={res.loading}
		error={res.error}
		onRetry={reloadAll}
		emptyIcon="ri:device-line"
		emptyMessage="No paired devices — run `virtues pair` on your server to pair this browser, or click Add device above."
		loadingMessage="Loading devices..."
		searchPlaceholder="Search by name, kind, version..."
		onRefresh={reloadAll}
		{rowIcon}
		{rowHref}
		onItemClick={openRow}
	>
		{#snippet tableRow(row: Row)}
			{@const device = row.device}
			<td class="px-3 py-2.5">
				<div class="flex items-center gap-2 flex-wrap" class:pl-5={device?.installed_by}>
					{#if device?.installed_by}
						<!-- Folded under the app that installed it (see the sort). -->
						<Icon
							icon="ri:corner-down-right-line"
							class="text-foreground-muted flex-shrink-0"
							width="14"
						/>
					{/if}
					<span class="text-sm font-medium text-foreground truncate">{row.label}</span>
					{#if device?.is_current}
						<Badge>This device</Badge>
					{/if}
				</div>
				<div class="text-xs text-foreground-muted mt-0.5" class:pl-5={device?.installed_by}>
					{row.kind}
				</div>
				{#if device}
					{#each deniedPermissions(device) as perm}
						<!-- A collector missing a permission isn't an error — nothing
						     crashed, and the rest of its streams are fine. It's a
						     capability the box has been quietly denied, so it reads as
						     a standing warning with the remedy attached, not a toast
						     that can be dismissed and forgotten. -->
						<div
							class="mt-2 flex items-start gap-2 rounded-md border border-warning/40 bg-warning/10 px-3 py-2 text-xs"
						>
							<Icon icon="ri:lock-line" class="text-warning mt-0.5 flex-shrink-0" />
							<div class="min-w-0">
								<span class="text-foreground font-medium">{perm.label} is off</span>
								<span class="text-foreground-muted"> — {perm.costs}.</span>
								{#if canFix && perm.open}
									<div class="text-foreground-muted mt-0.5">
										<!-- No "restart the collector". It re-checks on its own every
										     few minutes, so that instruction was jargon AND untrue —
										     it asked for work that was never needed. -->
										Turn on <span class="text-foreground">Virtues</span> in the list,
										then leave it — this Mac notices within a few minutes.
									</div>
									<button
										class="fix-btn mt-2"
										onclick={(e) => {
											// The whole row is the drill-down now; this must not also open it.
											e.stopPropagation();
											perm.open?.();
										}}
									>
										<Icon icon="ri:external-link-line" width="13" />
										Open {perm.label} on this Mac
									</button>
								{:else}
									<!-- Was unconditional, so a browser on a phone got told to
									     "turn on Virtues in the list" and that "this Mac notices
									     within a few minutes" — instructions for a machine the
									     reader is not at, and with no button beneath them,
									     because macOS forbids granting these remotely. Same
									     conditional the device page uses: two screens showing
									     one fact must not disagree about whether it is
									     actionable from here. -->
									<div class="text-foreground-muted mt-0.5">
										Granting this needs someone at that machine — macOS has no
										remote path for it.
									</div>
								{/if}
							</div>
						</div>
					{/each}
					{#if device.permissions?.stale}
						<div class="text-xs text-foreground-muted mt-2 italic">
							Permission report is stale — the collector may not be running.
						</div>
					{/if}
				{/if}
			</td>
			<td class="px-3 py-2.5">
				<div class="flex items-center gap-1.5">
					{#if row.behind}
						<!-- The mark is the whole point of putting the server in the
						     table: out of date reads the same on every row, and the
						     reason is one hover away. -->
						<span class="text-error flex-none inline-flex" title={row.behind}>
							<Icon icon="ri:arrow-up-circle-fill" width="15" />
						</span>
					{/if}
					{#if row.version}
						<span class="text-sm font-mono text-foreground">{row.version}</span>
					{:else if row.is_server && !update}
						<span class="text-xs text-foreground-muted">…</span>
					{:else}
						<span class="text-xs text-foreground-muted italic">version unknown</span>
					{/if}
				</div>
				{#if row.version_sub}
					<div class="text-xs text-foreground-muted mt-0.5 truncate">{row.version_sub}</div>
				{/if}
			</td>
			<td class="px-3 py-2.5 text-sm">
				{#if isConnected(row)}
					<span class="inline-flex items-center gap-2 text-foreground">
						<span class="dot dot-on"></span>Connected
					</span>
				{:else if row.last_seen_at}
					<span class="inline-flex items-center gap-2 text-foreground-muted">
						<span class="dot"></span>{formatDateTime(row.last_seen_at)}
					</span>
				{:else}
					<span class="inline-flex items-center gap-2 text-foreground-muted">
						<span class="dot"></span>Never
					</span>
				{/if}
			</td>
			<td class="px-3 py-2.5 text-sm text-foreground-muted hide-mobile">
				{#if row.paired_at}
					{formatDateTime(row.paired_at)}
					{#if device?.paired_from_ip}
						<div class="text-xs mt-0.5">from {device.paired_from_ip}</div>
					{/if}
				{:else}
					<span class="text-foreground-subtle">—</span>
				{/if}
			</td>
		{/snippet}

		{#snippet card(row: Row)}
			{@const device = row.device}
			<div class="flex flex-col items-center gap-2 text-center">
				<Icon icon={rowIcon(row)} class="text-3xl text-foreground-muted" />
				<span class="text-sm font-medium text-foreground break-all">{row.label}</span>
				<div class="flex items-center gap-1 flex-wrap justify-center">
					<Badge>{row.kind}</Badge>
					{#if device?.is_current}
						<Badge>This device</Badge>
					{/if}
				</div>
				{#if row.version}
					<span class="text-xs font-mono text-foreground inline-flex items-center gap-1">
						{#if row.behind}
							<span class="text-error inline-flex" title={row.behind}>
								<Icon icon="ri:arrow-up-circle-fill" width="13" />
							</span>
						{/if}
						{row.version}
					</span>
				{/if}
				<span class="text-xs text-foreground-muted inline-flex items-center gap-1.5">
					{#if isConnected(row)}
						<span class="dot dot-on"></span>Connected
					{:else if row.last_seen_at}
						<span class="dot"></span>{formatDateTime(row.last_seen_at)}
					{:else}
						<span class="dot"></span>Never seen
					{/if}
				</span>
				{#if device && deniedPermissions(device).length > 0}
					<span class="text-xs text-warning flex items-center gap-1">
						<Icon icon="ri:lock-line" width="12" />
						{deniedPermissions(device).length}
						{deniedPermissions(device).length === 1 ? "permission" : "permissions"} off
					</span>
				{/if}
			</div>
		{/snippet}

		{#snippet rowActions(row: Row)}
			{#if row.device}
				<Button variant="ghost" onclick={() => revoke(row.device!)}>
					<Icon icon="ri:close-circle-line" />
					Revoke
				</Button>
			{:else}
				<!-- The server cannot be revoked; its verb is an upgrade. -->
				<Button variant="ghost" onclick={() => openRow(row)}>
					<Icon icon="ri:download-cloud-2-line" />
					Updates
				</Button>
			{/if}
		{/snippet}
	</UniversalDataGrid>

	<!-- Revoke, in the plural. It lived at the foot of "Box" until 2026-08-17,
	     below the CPU graphs, which put the button that signs out every device
	     you own on a page you open to read a temperature. Its subject was
	     always this one: the same verb as the Revoke buttons above, applied to
	     all of them at once. -->
	<ReopenSection />
</Page>

{#if addOpen}
	<!-- Modal backdrop. Click outside dismisses; the inner stops propagation. -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm px-4"
		onclick={closeAdd}
		onkeydown={(e) => e.key === "Escape" && closeAdd()}
		role="dialog"
		tabindex="-1"
	>
		<div
			class="w-full max-w-md rounded-xl bg-surface border border-border shadow-xl p-6"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="document"
		>
			<div class="flex items-center justify-between mb-4">
				<h2 class="text-lg font-semibold">Add a device</h2>
				<button
					onclick={closeAdd}
					class="text-foreground-muted hover:text-foreground"
					aria-label="Close"
				>
					<Icon icon="ri:close-line" />
				</button>
			</div>

			{#if mintLoading}
				<LoadingState />
			{:else if mintError}
				<ErrorState message={mintError} />
			{:else if mintedUrl}
				<div class="space-y-4">
					{#if pairStatus === "authorized"}
						<div class="rounded-lg bg-surface-alt border border-border p-3 text-sm">
							<div class="flex items-center gap-2">
								<Icon icon="ri:loader-4-line" class="animate-spin text-foreground-muted" />
								<span class="text-foreground-muted">
									Waiting for the new device to open the link…
								</span>
							</div>
						</div>

						<div
							class="rounded-lg border border-border bg-white p-4 flex items-center justify-center"
						>
							{#if mintedQrSvg}
								<!-- Rendered server-side; the SVG is fully self-contained,
								     never touches a third party. -->
								<div class="w-56 h-56 [&_svg]:w-full [&_svg]:h-full">
									{@html mintedQrSvg}
								</div>
							{/if}
						</div>

						<div>
							<div class="text-xs text-foreground-muted mb-1">
								Or open this URL on the new device:
							</div>
							<div class="flex gap-2">
								<input
									readonly
									value={mintedUrl}
									class="flex-1 text-xs font-mono px-2 py-1.5 rounded border border-border bg-surface"
								/>
								<Button variant="ghost" onclick={copyUrl}>Copy</Button>
							</div>
						</div>
					{:else if pairStatus === "consumed"}
						<div class="rounded-lg bg-surface-alt border border-border p-4 text-sm flex items-start gap-3">
							<Icon icon="ri:check-line" class="text-success mt-0.5" />
							<div>
								<div class="font-medium">Device paired</div>
								{#if consumedByLabel}
									<div class="text-foreground-muted text-xs mt-0.5">
										{consumedByLabel}
									</div>
								{/if}
							</div>
						</div>
						<Button variant="primary" onclick={closeAdd} class="w-full">Done</Button>
					{:else if pairStatus === "expired"}
						<ErrorState message="This pair token expired. Close and try again." />
						<Button variant="ghost" onclick={closeAdd} class="w-full">Close</Button>
					{:else if pairStatus === "denied"}
						<div class="text-sm text-foreground-muted">Pair denied.</div>
						<Button variant="ghost" onclick={closeAdd} class="w-full">Close</Button>
					{/if}
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	/* The presence dot. Green = spoke to the box inside the window (or IS the
	   box, or is the device reading this); grey = a timestamp instead. */
	.dot {
		width: 8px;
		height: 8px;
		border-radius: 999px;
		flex: none;
		background: var(--color-foreground-subtle);
	}
	.dot-on {
		background: var(--color-success);
	}
	.fix-btn {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 3px 9px;
		border: 1px solid var(--color-border);
		border-radius: 6px;
		background: none;
		cursor: pointer;
		font-size: 12px;
		color: var(--color-foreground-muted);
	}

	.fix-btn:hover {
		background: color-mix(in srgb, var(--color-foreground) 8%, transparent);
		color: var(--color-foreground);
	}

	/* Matches the grid's own hideOnMobile header behavior for the paired-at
	   cell, which the custom tableRow has to hide itself. */
	@media (max-width: 768px) {
		.hide-mobile {
			display: none;
		}
	}
</style>
