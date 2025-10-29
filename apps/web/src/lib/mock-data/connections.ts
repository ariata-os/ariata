export interface Source {
	id: string;
	name: string;
	type: string;
	isActive: boolean;
	lastSyncAt: string | null;
	enabledStreamsCount: number;
	totalStreamsCount: number;
	createdAt: string;
}

export interface Stream {
	id: string;
	sourceId: string;
	streamName: string;
	displayName: string;
	description: string;
	tableName: string;
	isEnabled: boolean;
	cronSchedule: string | null;
	lastSyncAt: string | null;
	lastSyncStatus: "success" | "failed" | "partial" | "never" | null;
}

export interface SyncLog {
	id: string;
	sourceId: string;
	sourceName: string;
	streamName: string;
	streamDisplayName: string;
	startedAt: string;
	completedAt: string | null;
	durationMs: number | null;
	status: "success" | "failed" | "partial";
	recordsFetched: number | null;
	recordsWritten: number | null;
	recordsFailed: number | null;
	errorMessage: string | null;
}

export type ActivityType =
	| "sync"
	| "source_added"
	| "source_removed"
	| "stream_enabled"
	| "stream_disabled"
	| "token_refresh"
	| "transform"
	| "config_changed"
	| "cleanup";

export interface Activity {
	id: string;
	type: ActivityType;
	status: "success" | "failed" | "partial" | "in_progress";

	// Time tracking
	startedAt: string;
	completedAt: string | null;
	durationMs: number | null;

	// Context (varies by type)
	sourceId?: string | null;
	sourceName?: string | null;
	sourceType?: string | null;
	streamName?: string | null;
	streamDisplayName?: string | null;

	// Metrics (for syncs/transforms)
	recordsFetched?: number | null;
	recordsWritten?: number | null;
	recordsFailed?: number | null;

	// Description and errors
	description?: string;
	errorMessage?: string | null;

	// Metadata (type-specific data)
	metadata?: Record<string, any>;
}

// Mock Sources
export const mockSources: Source[] = [
	{
		id: "550e8400-e29b-41d4-a716-446655440001",
		name: "My Google Account",
		type: "google",
		isActive: true,
		lastSyncAt: new Date(Date.now() - 1000 * 60 * 45).toISOString(), // 45 min ago
		enabledStreamsCount: 2,
		totalStreamsCount: 2,
		createdAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 30).toISOString(), // 30 days ago
	},
	{
		id: "550e8400-e29b-41d4-a716-446655440002",
		name: "Personal iPhone",
		type: "ios",
		isActive: true,
		lastSyncAt: new Date(Date.now() - 1000 * 60 * 15).toISOString(), // 15 min ago
		enabledStreamsCount: 2,
		totalStreamsCount: 3,
		createdAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 15).toISOString(), // 15 days ago
	},
	{
		id: "550e8400-e29b-41d4-a716-446655440003",
		name: "Work Notion",
		type: "notion",
		isActive: true,
		lastSyncAt: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(), // 2 hours ago
		enabledStreamsCount: 1,
		totalStreamsCount: 1,
		createdAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 7).toISOString(), // 7 days ago
	},
];

// Mock Streams
export const mockStreams: Stream[] = [
	// Google streams
	{
		id: "stream-001",
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		streamName: "calendar",
		displayName: "Calendar",
		description: "Google Calendar events and meetings",
		tableName: "stream_google_calendar",
		isEnabled: true,
		cronSchedule: "0 */6 * * *", // Every 6 hours
		lastSyncAt: new Date(Date.now() - 1000 * 60 * 45).toISOString(),
		lastSyncStatus: "success",
	},
	{
		id: "stream-002",
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		streamName: "gmail",
		displayName: "Gmail",
		description: "Gmail messages and threads",
		tableName: "stream_google_gmail",
		isEnabled: true,
		cronSchedule: "*/15 * * * *", // Every 15 minutes
		lastSyncAt: new Date(Date.now() - 1000 * 60 * 45).toISOString(),
		lastSyncStatus: "success",
	},

	// iOS streams
	{
		id: "stream-003",
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		streamName: "healthkit",
		displayName: "HealthKit",
		description: "Health and fitness data from Apple Health",
		tableName: "stream_ios_healthkit",
		isEnabled: true,
		cronSchedule: "0 */1 * * *", // Every hour
		lastSyncAt: new Date(Date.now() - 1000 * 60 * 15).toISOString(),
		lastSyncStatus: "success",
	},
	{
		id: "stream-004",
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		streamName: "location",
		displayName: "Location",
		description: "Location history and significant places",
		tableName: "stream_ios_location",
		isEnabled: true,
		cronSchedule: "*/30 * * * *", // Every 30 minutes
		lastSyncAt: new Date(Date.now() - 1000 * 60 * 15).toISOString(),
		lastSyncStatus: "success",
	},
	{
		id: "stream-005",
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		streamName: "microphone",
		displayName: "Microphone",
		description: "Audio recordings and transcriptions",
		tableName: "stream_ios_microphone",
		isEnabled: false,
		cronSchedule: null,
		lastSyncAt: null,
		lastSyncStatus: "never",
	},

	// Notion streams
	{
		id: "stream-006",
		sourceId: "550e8400-e29b-41d4-a716-446655440003",
		streamName: "pages",
		displayName: "Pages",
		description: "Notion pages and databases",
		tableName: "stream_notion_pages",
		isEnabled: true,
		cronSchedule: "0 0 * * *", // Daily at midnight
		lastSyncAt: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
		lastSyncStatus: "success",
	},
];

// Mock Sync Logs
export const mockSyncLogs: SyncLog[] = [
	{
		id: "log-001",
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		sourceName: "My Google Account",
		streamName: "gmail",
		streamDisplayName: "Gmail",
		startedAt: new Date(Date.now() - 1000 * 60 * 45).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 44.5).toISOString(),
		durationMs: 30000,
		status: "success",
		recordsFetched: 127,
		recordsWritten: 127,
		recordsFailed: 0,
		errorMessage: null,
	},
	{
		id: "log-002",
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		sourceName: "My Google Account",
		streamName: "calendar",
		streamDisplayName: "Calendar",
		startedAt: new Date(Date.now() - 1000 * 60 * 45).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 44.8).toISOString(),
		durationMs: 12000,
		status: "success",
		recordsFetched: 42,
		recordsWritten: 42,
		recordsFailed: 0,
		errorMessage: null,
	},
	{
		id: "log-003",
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		sourceName: "Personal iPhone",
		streamName: "healthkit",
		streamDisplayName: "HealthKit",
		startedAt: new Date(Date.now() - 1000 * 60 * 15).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 14.8).toISOString(),
		durationMs: 12000,
		status: "success",
		recordsFetched: 354,
		recordsWritten: 354,
		recordsFailed: 0,
		errorMessage: null,
	},
	{
		id: "log-004",
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		sourceName: "Personal iPhone",
		streamName: "location",
		streamDisplayName: "Location",
		startedAt: new Date(Date.now() - 1000 * 60 * 15).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 14.9).toISOString(),
		durationMs: 6000,
		status: "success",
		recordsFetched: 18,
		recordsWritten: 18,
		recordsFailed: 0,
		errorMessage: null,
	},
	{
		id: "log-005",
		sourceId: "550e8400-e29b-41d4-a716-446655440003",
		sourceName: "Work Notion",
		streamName: "pages",
		streamDisplayName: "Pages",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 2 + 45000).toISOString(),
		durationMs: 45000,
		status: "success",
		recordsFetched: 89,
		recordsWritten: 89,
		recordsFailed: 0,
		errorMessage: null,
	},
	{
		id: "log-006",
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		sourceName: "My Google Account",
		streamName: "gmail",
		streamDisplayName: "Gmail",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 + 35000).toISOString(),
		durationMs: 35000,
		status: "partial",
		recordsFetched: 200,
		recordsWritten: 185,
		recordsFailed: 15,
		errorMessage: "Rate limit exceeded for 15 messages",
	},
	{
		id: "log-007",
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		sourceName: "Personal iPhone",
		streamName: "healthkit",
		streamDisplayName: "HealthKit",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 2).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 2 + 5000).toISOString(),
		durationMs: 5000,
		status: "failed",
		recordsFetched: 0,
		recordsWritten: 0,
		recordsFailed: 0,
		errorMessage: "Connection timeout: device not reachable",
	},
];

// Helper functions
export function getSourceById(id: string): Source | undefined {
	return mockSources.find((s) => s.id === id);
}

export function getStreamsBySourceId(sourceId: string): Stream[] {
	return mockStreams.filter((s) => s.sourceId === sourceId);
}

export function getSyncLogsBySourceId(sourceId: string, limit: number = 10): SyncLog[] {
	return mockSyncLogs
		.filter((log) => log.sourceId === sourceId)
		.sort((a, b) => new Date(b.startedAt).getTime() - new Date(a.startedAt).getTime())
		.slice(0, limit);
}

export function getAllSyncLogs(): SyncLog[] {
	return mockSyncLogs.sort(
		(a, b) => new Date(b.startedAt).getTime() - new Date(a.startedAt).getTime()
	);
}

export function getSourceTypeIcon(type: string): string {
	const icons: Record<string, string> = {
		google: "ri:google-fill",
		ios: "ri:apple-fill",
		notion: "simple-icons:notion",
		mac: "ri:macbook-line",
	};
	return icons[type] || "ri:database-2-line";
}

export function getSourceTypeColor(type: string): string {
	const colors: Record<string, string> = {
		google: "text-neutral-900",
		ios: "text-neutral-900",
		notion: "text-neutral-900",
		mac: "text-neutral-700",
	};
	return colors[type] || "text-neutral-600";
}

export function formatCronSchedule(cron: string | null): string {
	if (!cron) return "Manual";

	const scheduleMap: Record<string, string> = {
		"*/15 * * * *": "Every 15 minutes",
		"*/30 * * * *": "Every 30 minutes",
		"0 */1 * * *": "Every hour",
		"0 */6 * * *": "Every 6 hours",
		"0 0 * * *": "Daily at midnight",
		"0 9 * * 1": "Weekly on Monday at 9 AM",
	};

	return scheduleMap[cron] || cron;
}

// Mock Activities
export const mockActivities: Activity[] = [
	// Recent sync - Gmail
	{
		id: "activity-001",
		type: "sync",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 45).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 44.5).toISOString(),
		durationMs: 30000,
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		sourceName: "My Google Account",
		sourceType: "google",
		streamName: "gmail",
		streamDisplayName: "Gmail",
		recordsFetched: 127,
		recordsWritten: 127,
		recordsFailed: 0,
		errorMessage: null,
	},
	// Recent sync - Calendar
	{
		id: "activity-002",
		type: "sync",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 45).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 44.8).toISOString(),
		durationMs: 12000,
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		sourceName: "My Google Account",
		sourceType: "google",
		streamName: "calendar",
		streamDisplayName: "Calendar",
		recordsFetched: 42,
		recordsWritten: 42,
		recordsFailed: 0,
		errorMessage: null,
	},
	// Stream enabled
	{
		id: "activity-003",
		type: "stream_enabled",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 + 500).toISOString(),
		durationMs: 500,
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		sourceName: "Personal iPhone",
		sourceType: "ios",
		streamName: "microphone",
		streamDisplayName: "Microphone",
		errorMessage: null,
	},
	// Recent sync - HealthKit
	{
		id: "activity-004",
		type: "sync",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 15).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 14.8).toISOString(),
		durationMs: 12000,
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		sourceName: "Personal iPhone",
		sourceType: "ios",
		streamName: "healthkit",
		streamDisplayName: "HealthKit",
		recordsFetched: 354,
		recordsWritten: 354,
		recordsFailed: 0,
		errorMessage: null,
	},
	// Token refresh
	{
		id: "activity-005",
		type: "token_refresh",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 2 + 2500).toISOString(),
		durationMs: 2500,
		description: "Refreshed OAuth tokens",
		metadata: { sources_checked: 3, tokens_refreshed: 2 },
		errorMessage: null,
	},
	// Config changed
	{
		id: "activity-006",
		type: "config_changed",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 3).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 3 + 300).toISOString(),
		durationMs: 300,
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		sourceName: "My Google Account",
		sourceType: "google",
		streamName: "calendar",
		streamDisplayName: "Calendar",
		description: "Changed sync schedule to every 6 hours",
		errorMessage: null,
	},
	// Source added
	{
		id: "activity-007",
		type: "source_added",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 7).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 7 + 5000).toISOString(),
		durationMs: 5000,
		sourceId: "550e8400-e29b-41d4-a716-446655440003",
		sourceName: "Work Notion",
		sourceType: "notion",
		errorMessage: null,
	},
	// Sync - Notion Pages
	{
		id: "activity-008",
		type: "sync",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 2 + 45000).toISOString(),
		durationMs: 45000,
		sourceId: "550e8400-e29b-41d4-a716-446655440003",
		sourceName: "Work Notion",
		sourceType: "notion",
		streamName: "pages",
		streamDisplayName: "Pages",
		recordsFetched: 89,
		recordsWritten: 89,
		recordsFailed: 0,
		errorMessage: null,
	},
	// Stream disabled
	{
		id: "activity-009",
		type: "stream_disabled",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 12).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 12 + 400).toISOString(),
		durationMs: 400,
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		sourceName: "Personal iPhone",
		sourceType: "ios",
		streamName: "location",
		streamDisplayName: "Location",
		errorMessage: null,
	},
	// Partial sync
	{
		id: "activity-010",
		type: "sync",
		status: "partial",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 + 35000).toISOString(),
		durationMs: 35000,
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		sourceName: "My Google Account",
		sourceType: "google",
		streamName: "gmail",
		streamDisplayName: "Gmail",
		recordsFetched: 200,
		recordsWritten: 185,
		recordsFailed: 15,
		errorMessage: "Rate limit exceeded for 15 messages",
	},
	// Transform (future feature)
	{
		id: "activity-011",
		type: "transform",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 + 8000).toISOString(),
		durationMs: 8000,
		description: "Created location signals from GPS data",
		recordsWritten: 142,
		metadata: { signal_type: "location_clusters", input_records: 354 },
		errorMessage: null,
	},
	// Failed sync
	{
		id: "activity-012",
		type: "sync",
		status: "failed",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 2).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 2 + 5000).toISOString(),
		durationMs: 5000,
		sourceId: "550e8400-e29b-41d4-a716-446655440002",
		sourceName: "Personal iPhone",
		sourceType: "ios",
		streamName: "healthkit",
		streamDisplayName: "HealthKit",
		recordsFetched: 0,
		recordsWritten: 0,
		recordsFailed: 0,
		errorMessage: "Connection timeout: device not reachable",
	},
	// Cleanup
	{
		id: "activity-013",
		type: "cleanup",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 3).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 3 + 15000).toISOString(),
		durationMs: 15000,
		description: "Removed old activity logs (30+ days)",
		metadata: { records_deleted: 1247 },
		errorMessage: null,
	},
	// Source removed
	{
		id: "activity-014",
		type: "source_removed",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 15).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 15 + 3500).toISOString(),
		durationMs: 3500,
		sourceName: "Old iPhone",
		sourceType: "ios",
		errorMessage: null,
	},
	// Source added - initial
	{
		id: "activity-015",
		type: "source_added",
		status: "success",
		startedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 30).toISOString(),
		completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 30 + 6000).toISOString(),
		durationMs: 6000,
		sourceId: "550e8400-e29b-41d4-a716-446655440001",
		sourceName: "My Google Account",
		sourceType: "google",
		errorMessage: null,
	},
];

// Activity helper functions
export function getAllActivities(): Activity[] {
	return mockActivities.sort(
		(a, b) => new Date(b.startedAt).getTime() - new Date(a.startedAt).getTime()
	);
}

export function getActivityTypeIcon(type: ActivityType): string {
	const icons: Record<ActivityType, string> = {
		sync: "ri:download-cloud-line",
		source_added: "ri:add-circle-line",
		source_removed: "ri:delete-bin-line",
		stream_enabled: "ri:play-circle-line",
		stream_disabled: "ri:pause-circle-line",
		token_refresh: "ri:key-line",
		transform: "ri:git-branch-line",
		config_changed: "ri:settings-line",
		cleanup: "ri:broom-line",
	};
	return icons[type];
}

export function getActivityTypeColor(type: ActivityType): string {
	const colors: Record<ActivityType, string> = {
		sync: "text-blue-600",
		source_added: "text-green-600",
		source_removed: "text-red-600",
		stream_enabled: "text-emerald-600",
		stream_disabled: "text-orange-600",
		token_refresh: "text-amber-600",
		transform: "text-purple-600",
		config_changed: "text-neutral-600",
		cleanup: "text-neutral-500",
	};
	return colors[type];
}

export function getActivityTypeLabel(type: ActivityType): string {
	const labels: Record<ActivityType, string> = {
		sync: "Sync",
		source_added: "Source Added",
		source_removed: "Source Removed",
		stream_enabled: "Stream Enabled",
		stream_disabled: "Stream Disabled",
		token_refresh: "Token Refresh",
		transform: "Transformation",
		config_changed: "Configuration Update",
		cleanup: "Cleanup",
	};
	return labels[type];
}

export function formatDuration(ms: number | null): string {
	if (ms === null || ms === undefined) return "-";
	if (ms < 1000) return `${ms}ms`;
	if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
	if (ms < 3600000) return `${(ms / 60000).toFixed(1)}m`;
	return `${(ms / 3600000).toFixed(1)}h`;
}

export function getActivityDescription(activity: Activity): string {
	// If there's a custom description, use it
	if (activity.description) return activity.description;

	// Generate description based on type
	switch (activity.type) {
		case "sync":
			if (activity.recordsWritten !== null) {
				return `Synced ${activity.recordsWritten.toLocaleString()} records`;
			}
			return "Data sync completed";

		case "source_added":
			return `Added new source`;

		case "source_removed":
			return `Removed source`;

		case "stream_enabled":
			return activity.streamDisplayName
				? `Enabled ${activity.streamDisplayName} stream`
				: `Enabled stream`;

		case "stream_disabled":
			return activity.streamDisplayName
				? `Disabled ${activity.streamDisplayName} stream`
				: `Disabled stream`;

		case "token_refresh":
			const sources = activity.metadata?.sources_checked || 0;
			const refreshed = activity.metadata?.tokens_refreshed || sources;
			return `Refreshed ${refreshed} OAuth ${refreshed === 1 ? "token" : "tokens"}`;

		case "transform":
			return activity.metadata?.signal_type
				? `Created ${activity.metadata.signal_type} signals`
				: "Data transformation completed";

		case "config_changed":
			return activity.streamDisplayName
				? `Updated ${activity.streamDisplayName} configuration`
				: "Updated configuration";

		case "cleanup":
			const deleted = activity.metadata?.records_deleted || 0;
			return `Cleaned up ${deleted.toLocaleString()} old records`;

		default:
			return "System task";
	}
}
