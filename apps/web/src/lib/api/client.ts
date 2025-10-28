/**
 * API Client for Rust ELT Library
 *
 * All calls to /api/* are proxied to Rust backend (localhost:8000)
 * via Vite proxy (see vite.config.ts).
 *
 * Exception: /api/preferences and /api/dashboards are handled by SvelteKit
 * and use the Postgres app database (ariata_app).
 */

const API_BASE = '/api';

export interface Source {
	id: string;
	type: string;
	name: string;
	is_active: boolean;
	last_sync_at: string | null;
	created_at: string;
	updated_at: string;
	enabled_streams_count: number;
	total_streams_count: number;
}

export interface StreamInfo {
	name: string;
	display_name: string;
	description: string;
	table_name: string;
	is_enabled: boolean;
	supports_incremental: boolean;
	supports_full_refresh: boolean;
	cron_schedule: string | null;
	default_cron_schedule: string | null;
	last_sync_at: string | null;
	last_sync_status: string | null;
}

export interface SyncLog {
	id: string;
	source_id: string;
	stream_name: string;
	sync_mode: string;
	status: string;
	records_fetched: number | null;
	records_written: number | null;
	records_failed: number | null;
	duration_ms: number | null;
	error_message: string | null;
	started_at: string;
	completed_at: string | null;
}

export interface EnableStreamRequest {
	config?: Record<string, unknown>;
	cron_schedule?: string | null;
}

export interface SyncStreamRequest {
	mode?: 'incremental' | 'full_refresh';
}

// Sources
export async function listSources(): Promise<Source[]> {
	const res = await fetch(`${API_BASE}/sources`);
	if (!res.ok) throw new Error(`Failed to list sources: ${res.statusText}`);
	return res.json();
}

export async function getSource(sourceId: string): Promise<Source> {
	const res = await fetch(`${API_BASE}/sources/${sourceId}`);
	if (!res.ok) throw new Error(`Failed to get source: ${res.statusText}`);
	return res.json();
}

export async function deleteSource(sourceId: string): Promise<void> {
	const res = await fetch(`${API_BASE}/sources/${sourceId}`, {
		method: 'DELETE'
	});
	if (!res.ok) throw new Error(`Failed to delete source: ${res.statusText}`);
}

// Streams
export async function listStreams(sourceId: string): Promise<StreamInfo[]> {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/streams`);
	if (!res.ok) throw new Error(`Failed to list streams: ${res.statusText}`);
	return res.json();
}

export async function enableStream(
	sourceId: string,
	streamName: string,
	request: EnableStreamRequest = {}
): Promise<void> {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/streams/${streamName}/enable`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});
	if (!res.ok) throw new Error(`Failed to enable stream: ${res.statusText}`);
}

export async function disableStream(sourceId: string, streamName: string): Promise<void> {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/streams/${streamName}/disable`, {
		method: 'POST'
	});
	if (!res.ok) throw new Error(`Failed to disable stream: ${res.statusText}`);
}

export async function syncStream(
	sourceId: string,
	streamName: string,
	request: SyncStreamRequest = {}
): Promise<SyncLog> {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/streams/${streamName}/sync`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});
	if (!res.ok) throw new Error(`Failed to sync stream: ${res.statusText}`);
	return res.json();
}

// Sync Logs
export async function listSyncLogs(sourceId?: string, limit: number = 50): Promise<SyncLog[]> {
	const params = new URLSearchParams();
	if (sourceId) params.set('source_id', sourceId);
	params.set('limit', limit.toString());

	const res = await fetch(`${API_BASE}/logs?${params}`);
	if (!res.ok) throw new Error(`Failed to list sync logs: ${res.statusText}`);
	return res.json();
}

// OAuth
export function getOAuthUrl(sourceType: string, redirectUri: string): string {
	const params = new URLSearchParams({
		source_type: sourceType,
		redirect_uri: redirectUri
	});
	return `${API_BASE}/oauth/authorize?${params}`;
}

export async function handleOAuthCallback(
	code: string,
	state: string,
	sourceType: string
): Promise<Source> {
	const res = await fetch(`${API_BASE}/oauth/callback`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ code, state, source_type: sourceType })
	});
	if (!res.ok) throw new Error(`Failed to complete OAuth: ${res.statusText}`);
	return res.json();
}
