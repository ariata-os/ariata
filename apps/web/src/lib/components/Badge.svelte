<script lang="ts">
	let {
		variant = "default",
		size = "sm",
		class: className = "",
		children,
		...props
	}: any = $props();

	const variantStyles: any = {
		default: "bg-neutral-100 text-neutral-500 border-neutral-300",
		secondary: "bg-blue-50 text-blue-500 border-blue-200",
		success: "bg-orange-50 text-orange-500 border-orange-200",
		warning: "bg-yellow-50 text-yellow-500 border-yellow-200",
		danger: "bg-red-50 text-red-500 border-red-200",
		info: "bg-blue-50 text-blue-500 border-blue-200",
	};

	const sizeStyles: any = {
		sm: "px-2 py-0.5 text-xs",
		md: "px-2.5 py-1 text-sm",
		lg: "px-3 py-1.5 text-sm",
	};
</script>

<span
	class="inline-flex items-center font-medium rounded-full border {variantStyles[
		variant
	]} {sizeStyles[size]} {className}"
	{...props}
>
	{@render children()}
</span>
