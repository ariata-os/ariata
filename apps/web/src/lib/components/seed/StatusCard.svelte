<script lang="ts">
	import 'iconify-icon';

	export let title: string;
	export let value: number | string;
	export let description: string;
	export let icon: string;
	export let status: 'success' | 'warning' | 'error' | 'neutral' = 'neutral';

	const statusColors = {
		success: 'bg-green-50 text-green-600 border-green-200',
		warning: 'bg-yellow-50 text-yellow-600 border-yellow-200',
		error: 'bg-red-50 text-red-600 border-red-200',
		neutral: 'bg-neutral-50 text-neutral-600 border-neutral-200'
	};

	const iconColors = {
		success: 'text-green-500',
		warning: 'text-yellow-500',
		error: 'text-red-500',
		neutral: 'text-neutral-400'
	};
</script>

<div class="border rounded-lg p-6 {statusColors[status]}">
	<div class="flex items-start justify-between mb-3">
		<div class="flex items-center gap-3">
			<iconify-icon {icon} class="text-3xl {iconColors[status]}"></iconify-icon>
			<div>
				<h3 class="font-medium text-neutral-900 text-sm">{title}</h3>
				<p class="text-xs text-neutral-500 mt-0.5">{description}</p>
			</div>
		</div>
	</div>
	<div class="text-2xl font-semibold text-neutral-900">
		{value}
	</div>
</div>
