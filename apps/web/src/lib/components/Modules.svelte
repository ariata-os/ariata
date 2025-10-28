<script lang="ts">
    import "iconify-icon";

    interface Module {
        id: string;
        name: string;
        icon: string;
        iconFilled?: string;
        href?: string;
    }

    let { modules, activeModule, onModuleSelect, isSideNavOpen, toggleSubNav } =
        $props();

    let isLogoHovered = $state(false);

    // --- TRIANGLE VARIANTS (SubNav Closed) ---
    // Dot 1: Top center
    const triangle_dot1 = { x: "0%", y: "0%", left: "34%", top: "0%" };
    // Dot 2: Bottom left
    const triangle_dot2 = { x: "0%", y: "0%", left: "0%", top: "67%" };
    // Dot 3: Bottom right
    const triangle_dot3 = { x: "0%", y: "0%", left: "67%", top: "67%" };

    const triangle_dot1_hover = { ...triangle_dot1, y: "-4px" };
    const triangle_dot2_hover = { ...triangle_dot2, x: "-4px", y: "4px" };
    const triangle_dot3_hover = { ...triangle_dot3, x: "4px", y: "4px" };

    // --- INVERTED TRIANGLE VARIANTS (SubNav Open) ---
    // For clockwise rotation:
    // Dot 1 (top center) -> rotates to top right
    // Dot 2 (bottom left) -> rotates to top left
    // Dot 3 (bottom right) -> rotates to bottom center
    const inverted_triangle_dot1 = { x: "0%", y: "0%", left: "67%", top: "0%" };
    const inverted_triangle_dot2 = { x: "0%", y: "0%", left: "0%", top: "0%" };
    const inverted_triangle_dot3 = {
        x: "0%",
        y: "0%",
        left: "34%",
        top: "67%",
    };

    const inverted_triangle_dot1_hover = {
        ...inverted_triangle_dot1,
        x: "4px",
        y: "-4px",
    };
    const inverted_triangle_dot2_hover = {
        ...inverted_triangle_dot2,
        x: "-4px",
        y: "-4px",
    };
    const inverted_triangle_dot3_hover = {
        ...inverted_triangle_dot3,
        y: "4px",
    };

    function getDotAnimation(
        dotIndex: 1 | 2 | 3,
        subNavOpen: boolean,
        logoHovered: boolean,
    ): { x?: string; y?: string; left: string; top: string } {
        if (subNavOpen) {
            // TRIANGLE
            if (logoHovered) {
                if (dotIndex === 1) return triangle_dot1_hover;
                if (dotIndex === 2) return triangle_dot2_hover;
                if (dotIndex === 3) return triangle_dot3_hover;
            } else {
                if (dotIndex === 1) return triangle_dot1;
                if (dotIndex === 2) return triangle_dot2;
                if (dotIndex === 3) return triangle_dot3;
            }
        } else {
            // INVERTED TRIANGLE
            if (logoHovered) {
                if (dotIndex === 1) return inverted_triangle_dot1_hover;
                if (dotIndex === 2) return inverted_triangle_dot2_hover;
                if (dotIndex === 3) return inverted_triangle_dot3_hover;
            } else {
                if (dotIndex === 1) return inverted_triangle_dot1;
                if (dotIndex === 2) return inverted_triangle_dot2;
                if (dotIndex === 3) return inverted_triangle_dot3;
            }
        }
        return { left: "0%", top: "0%" }; // Should not happen
    }
</script>

<div class="flex h-full w-20 flex-col justify-between bg-neutral-100">
    <div>
        <!-- Ariata Animated Logo -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
            class="group relative flex h-16 w-full cursor-pointer items-center justify-center p-4 font-sans font-medium transition-all duration-300"
            onmouseenter={() => (isLogoHovered = true)}
            onmouseleave={() => (isLogoHovered = false)}
            onclick={toggleSubNav}
        >
            <div class="relative flex size-6 items-center justify-center">
                <div class="relative aspect-square h-[18px] w-[18px]">
                    <!-- Dot 1 -->
                    <div
                        class="absolute h-[6px] w-[6px] rounded-full bg-neutral-700 group-hover:bg-neutral-900 transition-all duration-200 ease-in-out"
                        style="transform: translate({getDotAnimation(
                            1,
                            isSideNavOpen,
                            isLogoHovered,
                        ).x || '0%'}, {getDotAnimation(
                            1,
                            isSideNavOpen,
                            isLogoHovered,
                        ).y || '0%'}); left: {getDotAnimation(
                            1,
                            isSideNavOpen,
                            isLogoHovered,
                        ).left}; top: {getDotAnimation(
                            1,
                            isSideNavOpen,
                            isLogoHovered,
                        ).top};"
                    ></div>

                    <!-- Dot 2 -->
                    <div
                        class="absolute h-[6px] w-[6px] rounded-full bg-neutral-700 group-hover:bg-neutral-900 transition-all duration-200 ease-in-out"
                        style="transform: translate({getDotAnimation(
                            2,
                            isSideNavOpen,
                            isLogoHovered,
                        ).x || '0%'}, {getDotAnimation(
                            2,
                            isSideNavOpen,
                            isLogoHovered,
                        ).y || '0%'}); left: {getDotAnimation(
                            2,
                            isSideNavOpen,
                            isLogoHovered,
                        ).left}; top: {getDotAnimation(
                            2,
                            isSideNavOpen,
                            isLogoHovered,
                        ).top};"
                    ></div>

                    <!-- Dot 3 -->
                    <div
                        class="absolute h-[6px] w-[6px] rounded-full bg-neutral-700 group-hover:bg-neutral-900 transition-all duration-200 ease-in-out"
                        style="transform: translate({getDotAnimation(
                            3,
                            isSideNavOpen,
                            isLogoHovered,
                        ).x || '0%'}, {getDotAnimation(
                            3,
                            isSideNavOpen,
                            isLogoHovered,
                        ).y || '0%'}); left: {getDotAnimation(
                            3,
                            isSideNavOpen,
                            isLogoHovered,
                        ).left}; top: {getDotAnimation(
                            3,
                            isSideNavOpen,
                            isLogoHovered,
                        ).top};"
                    ></div>
                </div>
            </div>
        </div>

        <!-- Module Icons -->
        {#each modules as module}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="group flex cursor-pointer items-center justify-center p-2"
                onclick={() => onModuleSelect(module.id)}
                role="button"
                tabindex="0"
                onkeypress={(e) => {
                    if (e.key === "Enter" || e.key === " ")
                        onModuleSelect(module.id);
                }}
            >
                <div
                    class="flex aspect-square w-full flex-col items-center justify-center rounded-xl transition-colors"
                    class:bg-neutral-200={activeModule === module.id}
                    class:hover:bg-neutral-200={activeModule !== module.id}
                >
                    <div class="mb-1.5 flex size-6 items-center justify-center">
                        <iconify-icon
                            icon={activeModule === module.id &&
                            module.iconFilled
                                ? module.iconFilled
                                : module.icon}
                            class="text-lg text-neutral-800 group-hover:text-neutral-800"
                        ></iconify-icon>
                    </div>
                    <span
                        class="text-xs whitespace-nowrap font-medium text-neutral-500 group-hover:text-neutral-900"
                    >
                        {module.name}
                    </span>
                </div>
            </div>
        {/each}
    </div>
</div>
