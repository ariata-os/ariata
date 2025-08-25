<script lang="ts">
    import { twMerge } from "tailwind-merge";

    const {
        href,
        variant = "filled",
        text,
        type = "button",
        target = "_self",
        disabled = false,
        icon = false,
        onclick,
        className = "",
    } = $props<{
        href?: string | null;
        variant?: "filled" | "outline" | "link" | "danger" | "text";
        target?: "_blank" | "_self";
        text: string;
        type?: "button" | "submit" | "link";
        icon?: string;
        onclick?: () => void;
        disabled?: boolean;
        className?: string;
    }>();

    // Define style classes based on variant
    const baseClasses =
        "text-sm w-fit h-fit rounded-lg relative overflow-hidden cursor-pointer font-sans w-auto min-w-fit whitespace-nowrap flex border border-transparent items-center justify-center px-4 py-2 transition-all duration-300 ease-in-out";

    // Define variant classes as an object
    const variantStyles: Record<string, string> = {
        filled: "bg-neutral-800 text-white hover:bg-gradient-to-br hover:from-blue-700 hover:via-blue-600 hover:to-indigo-400",
        text: "text-neutral-800 hover:text-neutral-900",
        outline:
            "border border-neutral-200 bg-white text-neutral-800 hover:bg-neutral-800 hover:text-white after:text-white",
        danger: "border border-rose-300 bg-white text-rose-600 hover:bg-rose-50 hover:border-rose-400 after:text-rose-600",
    };

    // Get the appropriate variant class
    const variantClasses = $derived(variantStyles[variant]);

    // Add disabled state classes
    const disabledClasses =
        "disabled:opacity-50 disabled:cursor-not-allowed disabled:pointer-events-none";

    // Merge classes, with variant styles taking priority over base classes
    const buttonClasses = $derived(
        twMerge(baseClasses, variantClasses, disabledClasses, className),
    );
</script>

{#if type === "submit" || type === "button"}
    <button {type} class={buttonClasses} {onclick} {disabled}>
        <span class="text-wrapper whitespace-nowrap" data-text={text}>
            <span class="text-content font-mono whitespace-nowrap">{text}</span>
        </span>
    </button>
{:else if type === "link"}
    <a
        {href}
        {target}
        class={buttonClasses}
        {onclick}
        role="button"
        aria-disabled={disabled}
        tabindex={disabled ? -1 : undefined}
        style={disabled ? "pointer-events: none; opacity: 0.65;" : ""}
    >
        <span class="text-wrapper whitespace-nowrap" data-text={text}>
            <span class="text-content font-mono whitespace-nowrap">{text}</span>
        </span>
    </a>
{/if}

<style>
    @reference '../../app.css';

    .text-wrapper {
        position: relative;
        display: inline-block;
        overflow: hidden;
    }

    .text-content {
        display: inline-block;
        position: relative;
        transition:
            transform 0.3s ease,
            opacity 0.3s ease;
    }

    /* Apply to both a and button hover states, but not when disabled */
    a:not([aria-disabled="true"]):hover .text-content,
    button:not(:disabled):hover .text-content {
        transform: translateY(-16px);
        opacity: 0;
    }

    .text-wrapper::after {
        content: attr(data-text);
        position: absolute;
        left: 0;
        top: 0;
        width: 100%;
        transform: translateY(16px);
        opacity: 0;
        transition:
            transform 0.3s ease,
            opacity 0.3s ease;
        @apply items-center font-mono;
    }

    /* Apply to both a and button hover states, but not when disabled */
    a:not([aria-disabled="true"]):hover .text-wrapper::after,
    button:not(:disabled):hover .text-wrapper::after {
        transform: translateY(0);
        opacity: 1;
    }

    /* Apply variant-specific after text colors */
    .after\:text-neutral-800 .text-wrapper::after {
        @apply text-neutral-800;
    }

    .after\:text-rose-600 .text-wrapper::after {
        @apply text-rose-600;
    }
    /* 
    .after\:text-white .text-wrapper::after {
        @apply text-white;
    } */
</style>
