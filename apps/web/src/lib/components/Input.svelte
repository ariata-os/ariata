<script lang="ts">
    interface InputProps {
        id?: string;
        type?: "text" | "number" | "email" | "password" | "url";
        value?: string;
        placeholder?: string;
        min?: number;
        max?: number;
        required?: boolean;
        disabled?: boolean;
        class?: string;
    }

    let {
        id,
        type = "text",
        value = $bindable(""),
        placeholder,
        min,
        max,
        required = false,
        disabled = false,
        class: className = "",
    }: InputProps = $props();
</script>

<input
    {id}
    {type}
    bind:value
    {placeholder}
    {min}
    {max}
    {required}
    {disabled}
    class="w-full px-3 py-1.5 text-sm border hover:border-neutral-400 border-neutral-200 bg-white rounded-lg focus:outline-none focus:ring-px focus:ring-neutral-600 focus:border-neutral-600 transition-all {className}"
/>
