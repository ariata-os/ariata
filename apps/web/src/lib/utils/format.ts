/**
 * Formatting utilities for dates, times, bytes, and other data
 */

/**
 * Format a date/time value to a locale string
 */
export function formatDate(date: string | Date | null): string {
	if (!date) return "—";
	const d = typeof date === "string" ? new Date(date) : date;
	return d.toLocaleString();
}

/**
 * Format a date/time as relative time (e.g., "2m ago", "3h ago")
 */
export function formatRelativeTime(date: string | Date | null): string {
	if (!date) return "Never";
	const d = typeof date === "string" ? new Date(date) : date;
	const now = new Date();
	const diff = now.getTime() - d.getTime();
	const minutes = Math.floor(diff / 60000);

	if (minutes < 1) return "Just now";
	if (minutes < 60) return `${minutes}m ago`;
	if (minutes < 1440) return `${Math.floor(minutes / 60)}h ago`;
	if (minutes < 10080) return `${Math.floor(minutes / 1440)}d ago`;

	return d.toLocaleDateString();
}

/**
 * Format the duration between two dates
 */
export function formatDuration(
	start: Date | string | null,
	end: Date | string | null,
): string {
	if (!start || !end) return "—";
	const startTime = typeof start === "string" ? new Date(start) : start;
	const endTime = typeof end === "string" ? new Date(end) : end;
	const seconds = (endTime.getTime() - startTime.getTime()) / 1000;

	if (seconds < 60) return `${Math.round(seconds)}s`;
	if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
	return `${Math.round(seconds / 3600)}h`;
}

/**
 * Format bytes to human-readable string
 */
export function formatBytes(bytes: number | null | undefined): string {
	if (bytes == null || bytes === 0 || isNaN(bytes)) return "0 B";
	const k = 1024;
	const sizes = ["B", "KB", "MB", "GB"];
	const i = Math.floor(Math.log(bytes) / Math.log(k));
	if (i < 0 || i >= sizes.length) return "0 B";
	return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

/**
 * Get status badge variant based on status string
 */
export function getStatusBadgeVariant(
	status: string | null | undefined,
): "success" | "error" | "warning" | "default" {
	switch (status?.toUpperCase()) {
		case "RUNNING":
		case "IN_PROGRESS":
			return "warning";
		case "COMPLETED":
		case "SUCCESS":
			return "success";
		case "FAILED":
		case "ERROR":
			return "error";
		default:
			return "default";
	}
}