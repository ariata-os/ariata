<script lang="ts">
    interface TextareaProps {
        id?: string;
        value?: string;
        placeholder?: string;
        rows?: number;
        required?: boolean;
        disabled?: boolean;
        class?: string;
    }
    
    let { 
        id, 
        value = $bindable(''), 
        placeholder, 
        rows = 4, 
        required = false, 
        disabled = false, 
        class: className = '', 
    }: TextareaProps = $props();
</script>

<textarea
    {id}
    bind:value
    {placeholder}
    {rows}
    {required}
    {disabled}
    class="w-full px-3 py-1.5 text-sm border hover:border-neutral-400 border-neutral-200 bg-white rounded-lg focus:outline-none focus:ring-px focus:ring-neutral-600 focus:border-neutral-600 transition-all resize-none {className}"
></textarea>