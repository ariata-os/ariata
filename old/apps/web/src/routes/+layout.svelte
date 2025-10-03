<script lang="ts">
    import "../app.css";

    import { page } from "$app/state";
    import { Toaster } from "svelte-sonner";
    import "iconify-icon";
    import { ModuleNav, SideNav, Breadcrumbs } from "$lib/components";
    import { goto, onNavigate } from "$app/navigation";
    import { setUserContext } from "$lib/stores/user";

    let { children, data } = $props();

    // Set user context for all child components
    if (data.user) {
        setUserContext(data.user);
    }

    let activeModule = $state("home");
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
            type?: "item" | "title";
        }>;
    }

    // Define modules
    const modules: Record<string, Module> = {
        home: {
            id: "home",
            name: "Home",
            icon: "ri:message-3-line",
            iconFilled: "ri:message-3-fill",
            title: "Home",
            items: [
                {
                    href: "/",
                    icon: "ri:message-3-line",
                    text: "New Chat",
                    pagespace: "",
                },
            ],
        },
        views: {
            id: "views",
            name: "Views",
            icon: "ri:eye-line",
            iconFilled: "ri:eye-fill",
            title: "Views",
            items: [
                {
                    href: "/views/timeline",
                    icon: "ri:calendar-todo-line",
                    text: "Timeline",
                    pagespace: "timeline",
                },
                {
                    href: "/views/location",
                    icon: "ri:road-map-line",
                    text: "Location",
                    pagespace: "location",
                },
                {
                    href: "/views/signal-analysis",
                    icon: "ri:pulse-line",
                    text: "Signal Analysis",
                    pagespace: "signal-analysis",
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
                // {
                //     href: "/data/overview",
                //     icon: "ri:file-list-line",
                //     text: "Overview",
                //     pagespace: "overview",
                // },
                // {
                // 	text: "Ingestion",
                // 	type: "title",
                // },
                {
                    href: "/data/sources",
                    icon: "ri:device-line",
                    text: "Sources",
                    pagespace: "sources",
                },
                {
                    href: "/data/raw",
                    icon: "ri:archive-2-line",
                    text: "Raw Storage",
                    pagespace: "raw",
                },
                {
                    href: "/data/pipeline",
                    icon: "ri:flow-chart",
                    text: "Pipeline",
                    pagespace: "pipeline",
                },
            ],
        },
        settings: {
            id: "settings",
            name: "Settings",
            icon: "ri:settings-3-line",
            iconFilled: "ri:settings-3-fill",
            title: "Settings",
            items: [
                {
                    href: "/settings/database",
                    icon: "ri:database-line",
                    text: "Database Access",
                    pagespace: "database",
                },
                // Future settings tabs can be added here:
                // {
                //     href: "/settings/api-keys",
                //     icon: "ri:key-line",
                //     text: "API Keys",
                //     pagespace: "api-keys",
                // },
                // {
                //     href: "/settings/general",
                //     icon: "ri:settings-line",
                //     text: "General",
                //     pagespace: "general",
                // },
            ],
        },
    };

    let currentModule = $derived.by(() => {
        const path = page.url.pathname.split("/")[1] || "";

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

        return "home"; // Default to home if no match
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

    onNavigate((navigation) => {
        // @ts-ignore
        if (!document.startViewTransition) return;

        return new Promise((resolve) => {
            // @ts-ignore
            document.startViewTransition(async () => {
                resolve();
                await navigation.complete;
            });
        });
    });
</script>

<Toaster position="top-center" />

<div class="flex h-screen w-full bg-white">
    <!-- Module Navigation (Left Sidebar) -->
    <div id="module-nav" class="z-20 h-full border-r border-neutral-200">
        <ModuleNav
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
        <SideNav
            items={modules[activeModule as keyof typeof modules]?.items || []}
            moduleTitle={modules[activeModule as keyof typeof modules]?.title ||
                ""}
            class="h-full w-56"
        ></SideNav>
    </div>

    <!-- Main Content -->
    <main
        class="flex-1 flex flex-col z-0 bg-neutral-100 text-neutral-900 transition-all duration-300 min-w-0"
    >
        <header
            class="{isSideNavOpen
                ? 'h-16 opacity-100'
                : 'h-0 opacity-0'} bg-neutral-100 w-full transition-all duration-300 flex items-center px-6"
        >
            <Breadcrumbs />
        </header>
        <div
            class="border-t border-r border-b overflow-auto {!isSideNavOpen
                ? ''
                : 'border-l'} flex-1 bg-white border-neutral-200 transition-all duration-300 {isSideNavOpen
                ? 'rounded-tl-3xl'
                : 'rounded-none'} min-w-0"
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
