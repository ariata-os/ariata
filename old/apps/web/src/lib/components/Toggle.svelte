<script lang="ts">
	interface Props {
		checked?: boolean;
		disabled?: boolean;
		onChange?: (checked: boolean) => void;
		label?: string;
		size?: 'sm' | 'md' | 'lg';
	}

	let { 
		checked = $bindable(false), 
		disabled = false, 
		onChange,
		label,
		size = 'md'
	}: Props = $props();

	function handleToggle() {
		if (!disabled) {
			checked = !checked;
			onChange?.(checked);
		}
	}

	const sizeClasses = {
		sm: 'h-4 w-8',
		md: 'h-5 w-10',
		lg: 'h-6 w-12'
	};

	const dotSizeClasses = {
		sm: 'h-3 w-3',
		md: 'h-3.5 w-3.5', 
		lg: 'h-4 w-4'
	};

	const dotTranslateClasses = {
		sm: 'translate-x-4',
		md: 'translate-x-5',
		lg: 'translate-x-6'
	};
</script>

<button
	type="button"
	role="switch"
	aria-checked={checked}
	disabled={disabled}
	onclick={handleToggle}
	class="group relative inline-flex items-center"
>
	<span
		class="relative inline-flex {sizeClasses[size]} flex-shrink-0 cursor-pointer items-center rounded-full transition-all duration-200 ease-in-out focus:outline-none focus:ring-1 focus:ring-neutral-400 focus:ring-offset-1 {disabled
			? 'cursor-not-allowed opacity-40'
			: 'hover:shadow-sm'} {checked ? 'bg-neutral-800' : 'bg-neutral-200 hover:bg-neutral-300'}"
	>
		<span
			class="pointer-events-none inline-block {dotSizeClasses[size]} transform rounded-full bg-white shadow-sm ring-0 transition-transform duration-200 ease-in-out {checked
				? dotTranslateClasses[size]
				: 'translate-x-0.5'}"
		/>
	</span>
	{#if label}
		<span class="ml-3 text-sm font-medium text-neutral-900 {disabled ? 'opacity-50' : ''}">
			{label}
		</span>
	{/if}
</button>