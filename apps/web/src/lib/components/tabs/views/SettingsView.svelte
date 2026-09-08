<!--
	SettingsView.svelte

	The one Settings room. A single door in the sidebar opens here; every
	preference and console lives under it as a route-driven section. Replaces the
	former two rooms (Account + System) and the flat footer folder
	(Sources · Tools · Profile · Devices · System · Developers · Sign Out).

	Doctrine: one tab group max. The primary sub-nav (the nouns the user owns) is
	the only nav for most sections; only Developer carries a secondary sub-nav,
	and nothing nests a tab group inside a tab group.

	  You          /virtues/you            — profile, theme
	  Assistant    /virtues/assistant      — name, persona, model
	  Billing      /virtues/billing        — subscription, balance, routing, and
	                                         the AI calls that draw it down
	  System       /virtues/system         — the machine, measured (read-only)
	  Network      /virtues/network        — which network the box is on
	  Software     /virtues/software       — release, track, update, artifacts
	  Devices      /virtues/devices        — paired devices (Unpair, Start over)
	               /virtues/devices/this   — the machine you're on (local panel)
	               /virtues/devices/:id    — another device, as the box knows it
	  Display      /virtues/display        — the screen on the box, and what it shows
	  Developer    /virtues/developer      — SQL · Terminal · Lake

	"Box" was three of those in one door (2026-08-17). It stacked a Wi-Fi
	picker, an update installer, an 8-chapter telemetry console and a
	revoke-every-device button on a single scroll — a reading, two settings and
	a detonator sharing one scrollbar — and the console's own "Network &
	Devices" and "About" chapters answered questions the two sections above
	them had already answered differently. Splitting it by subject is what
	removes the duplication; it was never really one room.

	There is no "Sign out" — auth is the device's proven iroh key, not a server
	session. The destructive action is Unpair, and it lives next to the thing it
	destroys (Devices) — which is also why "Start over", the plural of Unpair,
	moved there out of Box.

	Legacy flat/nested routes (/virtues/account, /virtues/connections/*,
	/virtues/box/devices, /virtues/developer/console, …) self-heal on mount.
-->

<script lang="ts">
	import type { Tab } from '$lib/tabs/types';
	import { windowShellStore } from '$lib/stores/window-shell.svelte';
	import { isMacOS, isIOS } from '$lib/utils/platform';
	import SoftwareView from '$lib/components/tabs/views/SoftwareView.svelte';

	import ProfileView from '$lib/components/tabs/views/ProfileView.svelte';
	import AssistantView from '$lib/components/tabs/views/AssistantView.svelte';
	import ModelsView from '$lib/components/tabs/views/ModelsView.svelte';
	import BillingView from '$lib/components/tabs/views/BillingView.svelte';
	import SystemInfoView from '$lib/components/tabs/views/SystemInfoView.svelte';
	import DevicesView from '$lib/components/tabs/views/DevicesView.svelte';
	import DisplayView from '$lib/components/tabs/views/DisplayView.svelte';
	import DeviceView from '$lib/components/tabs/views/DeviceView.svelte';
	import ThisDeviceView from '$lib/components/tabs/views/ThisDeviceView.svelte';
	import DeveloperSqlView from '$lib/components/tabs/views/DeveloperSqlView.svelte';
	import DeveloperTerminalView from '$lib/components/tabs/views/DeveloperTerminalView.svelte';
	import DeveloperLakeView from '$lib/components/tabs/views/DeveloperLakeView.svelte';

	let { tab, active }: { tab: Tab; active: boolean } = $props();

	// Old routes → their home in the flattened room. Rewritten in place so the
	// sub-nav underline and the content both land on the canonical section.
	const LEGACY_ROUTES: Record<string, string> = {
		'/virtues': '/virtues/you',
		'/virtues/account': '/virtues/you',
		'/virtues/profile': '/virtues/you',
		'/virtues/account/assistant': '/virtues/assistant',
		// Billing and Usage merged into Plan (2026-08-31): one subject seen from
		// two ends — what you are on, what is left, what drew it down. Both of
		// their own doors redirect too, because they were live sections with
		// bookmarks, not just historical paths.
		'/virtues/usage': '/virtues/billing',
		'/virtues/account/billing': '/virtues/billing',
		'/virtues/byo-key': '/virtues/billing',
		'/virtues/billing/plan': '/virtues/billing',
		'/virtues/account/usage': '/virtues/billing',
		'/virtues/billing/usage': '/virtues/billing',
		// Telemetry was this page under a word for something you send somewhere.
		'/virtues/telemetry': '/virtues/billing',
		'/virtues/developer/telemetry': '/virtues/billing',
		// "Plan" named a choice that does not exist — one subscription, one
		// price. Short-lived (2026-08-31 → 09-03), but it was a live row with
		// bookmarks, so its door stays open.
		'/virtues/plan': '/virtues/billing',
		// The 24h readings went back to System (2026-09-03): they are the
		// machine measured, and had no business under a wallet balance.
		'/virtues/system/history': '/virtues/system',
		// Network, Display and Software stopped being top-level sections
		// (2026-08-31). Network and Display are the machine's own connection and
		// its own screen, so they live under System; Software described the
		// release a DEVICE runs, so it became that device's page under Devices.
		'/virtues/network': '/virtues/system',
		'/virtues/software': '/virtues/devices/server',
		// Box split into System · Network · Software (2026-08-17). Its own door
		// lands on System, the largest of the three and the one that kept the
		// page's former title.
		'/virtues/box': '/virtues/system',
		'/virtues/box/health': '/virtues/system',
		'/virtues/box/network': '/virtues/system',
		'/virtues/box/updates': '/virtues/devices/server',
		'/virtues/updates': '/virtues/devices/server',
		'/virtues/version': '/virtues/devices/server',
		// Devices absorbed both Start over (the split above) and This Mac, which
		// was a top-level section for one instance of hardware. It is now
		// `devices/this` — a device page like any other, and a stable word
		// Sources can link to without knowing a device id.
		'/virtues/box/devices': '/virtues/devices',
		'/virtues/system/devices': '/virtues/devices',
		'/virtues/devices/': '/virtues/devices',
		'/virtues/this-mac': '/virtues/devices/this',
		'/virtues/box/this-mac': '/virtues/devices/this',
		'/virtues/system/this-mac': '/virtues/devices/this',
		// Developer flattened — Console layer removed
		'/virtues/developer': '/virtues/developer/sql',
		'/virtues/developer/console': '/virtues/developer/sql',
		// The pre-Settings developer doors (/virtues/sql, and the /developers/*
		// group that used to self-heal them). One room now.
		'/virtues/sql': '/virtues/developer/sql',
		'/virtues/terminal': '/virtues/developer/terminal',
		'/virtues/lake': '/virtues/developer/lake',
		// The auth-activity log is gone. Its old doors land on Devices, which is
		// where the thing it reported on — what is paired, and what you can
		// revoke — actually lives.
		'/virtues/system/activity': '/virtues/devices',
		'/virtues/activity': '/virtues/devices',
		'/virtues/developer/activity': '/virtues/devices',
	};

	// Sources left Settings for its own door, so these can't be rewritten in
	// place the way the others are — `/sources` is a different tab type, and
	// only `navigate` re-resolves that through the registry.
	const SOURCES_ALIASES = [
		'/virtues/sources',
		'/virtues/connections',
		'/virtues/connections/sources',
		'/virtues/connections/tools',
		'/virtues/tools',
	];

	$effect(() => {
		if (SOURCES_ALIASES.includes(tab.route)) {
			windowShellStore.navigate('/sources', { label: 'Sources' });
			return;
		}
		const next = LEGACY_ROUTES[tab.route];
		if (next) windowShellStore.updateTab(tab.id, { route: next });
	});

	type Section =
		| 'you'
		| 'assistant'
		| 'models'
		| 'billing'
		| 'system'
		| 'devices'
		| 'display'
		| 'developer';

	const SECTIONS = [
		'assistant',
		'models',
		'billing',
		'system',
		'devices',
		'display',
		'developer',
	];

	// (section, sub) are a pure function of the route. `raw` is everything after
	// `/virtues/`; segment 1 = section, segment 2 = sub-section (Developer only).
	const raw = $derived(tab.route.replace(/^\/virtues\/?/, ''));
	const section = $derived<Section>(
		SECTIONS.includes(raw.split('/')[0]) ? (raw.split('/')[0] as Section) : 'you'
	);
	const sub = $derived(raw.split('/')[1] ?? '');


</script>

<!--
	No SubNav. The sidebar carries this nav now (lib/sidebar/modes.ts) — keeping
	the horizontal row too would mean two navigations for one set of sections,
	side by side, disagreeing about which is in charge. Removing it is also what
	retires the second underline row Developer used to add, which was the
	original complaint.
-->
<div class="settings-view">
	<main class="content">
		{#if section === 'you'}
			<ProfileView {tab} {active} />
		{:else if section === 'assistant'}
			<AssistantView {tab} {active} />
		{:else if section === 'models'}
			<ModelsView {tab} {active} />
		{:else if section === 'billing'}
			<BillingView {tab} {active} />
		{:else if section === 'system'}
			<SystemInfoView {tab} {active} />
		{:else if section === 'display'}
			<DisplayView />
		{:else if section === 'devices'}
			{#if !sub}
				<DevicesView {tab} {active} />
			{:else if sub === 'server'}
				<!-- The server's own page. It is row zero of Devices, not a
				     separate subject: "what release does this machine run" is a
				     fact about a device. -->
				<SoftwareView />
			{:else if sub === 'this' && isIOS}
				<!-- iOS keeps its own instrument panel for now: MobileDeviceScreen
				     is 900 lines of stream-by-stream local detail that has no
				     box-side equivalent. iOS FIRST, because an iPad satisfies
				     `isMacOS` too (it reports a desktop platform string; see
				     utils/platform.ts) and would otherwise get the Mac panel. -->
				<ThisDeviceView />
			{:else}
				<!-- ONE page for every other device, including this Mac. It used
				     to be three, split by vantage — what the box knows vs what the
				     device knows — with nothing on screen saying that was the axis,
				     so permissions and versions appeared twice and disagreed for
				     reasons no reader could see. DeviceView states the vantage per
				     section instead, and reads the local daemon when it is
				     standing on the machine it describes. -->
				<DeviceView deviceId={sub} {tab} {active} />
			{/if}
		{:else if section === 'developer'}
			{#if sub === 'terminal'}
				<DeveloperTerminalView {tab} {active} />
			{:else if sub === 'lake'}
				<DeveloperLakeView {tab} {active} />
			{:else}
				<DeveloperSqlView {tab} {active} />
			{/if}
		{/if}
	</main>
</div>

<style>
	.settings-view {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}

	.content {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
	}
</style>
