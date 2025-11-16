<script lang="ts">
	import "../app.css";
	import { page } from "$app/state";
	import { Toaster } from "svelte-sonner";
	import "iconify-icon";
	import { Modules, Sidebar, Breadcrumbs } from "$lib";
	import { chatSessions } from "$lib/stores/chatSessions.svelte";
	import { goto, onNavigate } from "$app/navigation";
	import { onMount } from "svelte";
	import { createAIContext } from '@ai-sdk/svelte';

	let { children, data } = $props();

	// Create AI context for synchronized state across Chat instances
	createAIContext();

	let activeModule = $state("chat");
	let isSideNavOpen = $state(true);

	interface Module {
		id: string;
		name: string;
		icon: string;
		iconFilled: string;
		title: string;
		paths?: string[];
		items?: Array<{
			href?: string;
			icon?: string;
			text: string;
			pagespace?: string;
			type?: "item" | "title" | "action";
			onclick?: () => void;
		}>;
	}

	// Define modules
	const modules: Record<string, Module> = {
		chat: {
			id: "chat",
			name: "Chat",
			icon: "ri:message-3-line",
			iconFilled: "ri:message-3-fill",
			title: "Chat",
			items: [
				{
					href: "/",
					icon: "ri:message-3-line",
					text: "Chat",
					pagespace: "",
				},
			],
		},
		data: {
			id: "data",
			name: "Data",
			icon: "ri:database-2-line",
			iconFilled: "ri:database-2-fill",
			title: "Data",
			items: [
				{
					href: "/data/sources",
					icon: "ri:device-line",
					text: "Sources",
					pagespace: "data/sources",
				},
				{
					href: "/data/ontologies",
					icon: "ri:node-tree",
					text: "Ontologies",
					pagespace: "data/ontologies",
				},
				{
					href: "/data/jobs",
					icon: "ri:history-line",
					text: "Activity",
					pagespace: "data/jobs",
				},
			],
		},
		profile: {
			id: "profile",
			name: "Profile",
			icon: "ri:user-line",
			iconFilled: "ri:user-fill",
			title: "Profile",
			items: [
				{
					href: "/profile/account",
					icon: "ri:user-line",
					text: "Account",
					pagespace: "account",
				},
				{
					href: "/profile/assistant",
					icon: "ri:robot-line",
					text: "Assistant",
					pagespace: "assistant",
				},
			],
		},
		axiology: {
			id: "axiology",
			name: "Axiology",
			icon: "ri:compass-3-line",
			iconFilled: "ri:compass-3-fill",
			title: "Axiology",
			items: [
				{
					href: "/axiology",
					icon: "ri:compass-3-line",
					text: "Overview",
				},
				{
					href: "/axiology/tasks",
					icon: "ri:checkbox-line",
					text: "Tasks",
				},
			],
		},
	};

	let currentModule = $derived.by(() => {
		const path = page.url.pathname.split("/")[1] || "";

		// Special handling for root-level chat routes
		// Root (/) and /[conversationId] are chat
		if (
			path === "" ||
			!["data", "profile", "axiology", "api", "oauth"].includes(path)
		) {
			// If it's not one of the known modules, assume it's a conversation ID (chat)
			return "chat";
		}

		// Find which module contains this path
		for (const [moduleId, module] of Object.entries(modules)) {
			if (module.paths?.includes(path)) {
				return moduleId;
			}
			// For modules without paths, check if the path matches the module id
			if (!module.paths && moduleId === path) {
				return moduleId;
			}
		}

		return "chat"; // Default to chat if no match
	});

	// Update active module when page changes
	$effect(() => {
		activeModule = currentModule;
	});

	function toggleSubNav() {
		isSideNavOpen = !isSideNavOpen;
	}

	function handleModuleSelect(moduleId: string) {
		activeModule = moduleId;
		// Navigate to the first item in the module, or the module root if no items
		const module = modules[moduleId as keyof typeof modules];
		const firstItem = module?.items?.find((item) => item.type !== "title");
		if (firstItem?.href) {
			goto(firstItem.href);
		} else {
			// Navigate to module root if no items
			goto(`/${moduleId}`);
		}
		// Optionally, ensure subnav opens when a module is selected if it was closed
		if (!isSideNavOpen) {
			isSideNavOpen = true;
		}
	}

	// Load chat sessions on mount
	onMount(() => {
		chatSessions.load();
	});

	// Create dynamic chat items from loaded sessions
	const chatItems = $derived.by(() => {
		const items: Module["items"] = [
			{
				text: "New Chat",
				icon: "ri:add-line",
				onclick: () => {
					goto("/");
				},
				type: "action" as const,
			},
		];

		// Add loaded sessions as navigation items
		for (const session of chatSessions.sessions) {
			items.push({
				href: `/?conversationId=${session.conversation_id}`,
				text: session.title || "Untitled",
				pagespace: session.conversation_id,
				icon: "ri:message-3-line",
			});
		}

		return items;
	});
</script>

<Toaster position="top-center" />

<div class="flex h-screen w-full">
	<!-- Module Navigation (Left Sidebar) -->
	<div id="module-nav" class="z-20 h-full border-r border-stone-200">
		<Modules
			modules={Object.values(modules)}
			{activeModule}
			onModuleSelect={handleModuleSelect}
			{isSideNavOpen}
			{toggleSubNav}
		/>
	</div>

	<!-- Sub Navigation (Module-specific) -->
	<div
		id="side-nav"
		class="h-full overflow-hidden transition-all duration-300 ease-in-out"
		style="width: {isSideNavOpen ? '14rem' : '0'}"
	>
		<Sidebar
			items={activeModule === "chat"
				? chatItems
				: modules[activeModule as keyof typeof modules]?.items || []}
			moduleTitle={modules[activeModule as keyof typeof modules]?.title ||
				""}
			class="h-full w-56"
		></Sidebar>
	</div>

	<!-- Main Content -->
	<main
		class="flex-1 flex flex-col z-0 transition-all duration-300 min-w-0 bg-paper-dark text-stone-800"
	>
		<header
			class="{isSideNavOpen
				? 'h-16 opacity-100'
				: 'h-0 opacity-0'} w-full transition-all duration-300 flex items-center px-6 bg-paper-dark"
		>
			<Breadcrumbs />
		</header>
		<div
			class="border-t border-r border-b {!isSideNavOpen
				? ''
				: 'border-l'} flex-1 transition-all duration-300 {isSideNavOpen
				? 'rounded-tl-3xl'
				: 'rounded-none'} min-w-0 overflow-hidden border-stone-200"
		>
			{@render children()}
		</div>
	</main>
</div>

<style>
	header {
		view-transition-name: main-header;
	}

	#module-nav {
		view-transition-name: module-nav;
	}

	#side-nav {
		view-transition-name: side-nav;
	}
</style>
