<script lang="ts">
	interface RadioProps {
		name: string;
		value: string;
		checked?: boolean;
		disabled?: boolean;
		label?: string;
		description?: string;
		onchange?: (e: Event) => void;
	}

	let {
		name,
		value,
		checked = false,
		disabled = false,
		label,
		description,
		onchange,
	}: RadioProps = $props();
</script>

<label class="flex items-start cursor-pointer {disabled ? 'opacity-50 cursor-not-allowed' : ''}">
	<div class="flex items-center h-5">
		<input
			type="radio"
			{name}
			{value}
			{checked}
			{disabled}
			{onchange}
			class="w-4 h-4 text-neutral-600 bg-white border-neutral-300 focus:ring-neutral-500 focus:ring-2 focus:ring-offset-0"
			style="accent-color: rgb(82, 82, 82);"
		/>
	</div>
	{#if label || description}
		<div class="ml-3">
			{#if label}
				<span class="text-sm font-medium text-neutral-700 {disabled ? 'text-neutral-400' : ''}">
					{label}
				</span>
			{/if}
			{#if description}
				<p class="text-sm text-neutral-500 {disabled ? 'text-neutral-400' : ''}">
					{description}
				</p>
			{/if}
		</div>
	{/if}
</label>

<style>
	/* Override default browser radio button styling */
	input[type="radio"] {
		appearance: none;
		-webkit-appearance: none;
		-moz-appearance: none;
		width: 1rem;
		height: 1rem;
		border: 1.5px solid rgb(212, 212, 212);
		border-radius: 50%;
		background-color: white;
		cursor: pointer;
		position: relative;
		transition: all 0.15s ease;
	}

	input[type="radio"]:checked {
		border-color: rgb(82, 82, 82);
		background-color: rgb(82, 82, 82);
	}

	input[type="radio"]:checked::after {
		content: '';
		display: block;
		width: 4px;
		height: 4px;
		border-radius: 50%;
		background-color: white;
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
	}

	input[type="radio"]:hover:not(:disabled) {
		border-color: rgb(115, 115, 115);
	}

	input[type="radio"]:focus {
		outline: none;
		box-shadow: 0 0 0 2px rgba(82, 82, 82, 0.1);
	}

	input[type="radio"]:disabled {
		cursor: not-allowed;
		opacity: 0.5;
	}
</style>