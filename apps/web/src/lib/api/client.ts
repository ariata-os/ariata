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

// Catalog
export async function listCatalogSources() {
	const res = await fetch(`${API_BASE}/catalog/sources`);
	if (!res.ok) throw new Error(`Failed to list catalog sources: ${res.statusText}`);
	return res.json();
}

// Sources
export async function listSources() {
	const res = await fetch(`${API_BASE}/sources`);
	if (!res.ok) throw new Error(`Failed to list sources: ${res.statusText}`);
	return res.json();
}

export async function getSource(sourceId: string) {
	const res = await fetch(`${API_BASE}/sources/${sourceId}`);
	if (!res.ok) throw new Error(`Failed to get source: ${res.statusText}`);
	return res.json();
}

export async function pauseSource(sourceId: string) {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/pause`, {
		method: 'POST'
	});
	if (!res.ok) throw new Error(`Failed to pause source: ${res.statusText}`);
	return res.json();
}

export async function resumeSource(sourceId: string) {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/resume`, {
		method: 'POST'
	});
	if (!res.ok) throw new Error(`Failed to resume source: ${res.statusText}`);
	return res.json();
}

export async function deleteSource(sourceId: string) {
	const res = await fetch(`${API_BASE}/sources/${sourceId}`, {
		method: 'DELETE'
	});
	if (!res.ok) throw new Error(`Failed to delete source: ${res.statusText}`);
}

// Streams
export async function listStreams(sourceId: string) {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/streams`);
	if (!res.ok) throw new Error(`Failed to list streams: ${res.statusText}`);
	return res.json();
}

export async function enableStream(
	sourceId: string,
	streamName: string,
	request: any = {}
) {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/streams/${streamName}/enable`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});
	if (!res.ok) throw new Error(`Failed to enable stream: ${res.statusText}`);
}

export async function disableStream(sourceId: string, streamName: string) {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/streams/${streamName}/disable`, {
		method: 'POST'
	});
	if (!res.ok) throw new Error(`Failed to disable stream: ${res.statusText}`);
}

export async function syncStream(
	sourceId: string,
	streamName: string,
	request: any = {}
): Promise<{ job_id: string; status: string; started_at: string }> {
	const res = await fetch(`${API_BASE}/sources/${sourceId}/streams/${streamName}/sync`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});
	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to sync stream: ${res.statusText}`);
	}
	return res.json();
}

// Jobs
export interface Job {
	id: string;
	job_type: 'sync' | 'transform';
	status: 'pending' | 'running' | 'succeeded' | 'failed' | 'cancelled';
	source_id?: string;
	source_name?: string; // Enriched in page load, not from API
	stream_name?: string;
	sync_mode?: 'full_refresh' | 'incremental';
	transform_id?: string;
	started_at: string;
	completed_at?: string;
	records_processed: number;
	error_message?: string;
	error_class?: string;
	metadata: any;
	created_at: string;
	updated_at: string;
}

export async function getJobStatus(jobId: string): Promise<Job> {
	const res = await fetch(`${API_BASE}/jobs/${jobId}`);
	if (!res.ok) throw new Error(`Failed to get job status: ${res.statusText}`);
	return res.json();
}

export async function queryJobs(params: {
	source_id?: string;
	status?: string[]; // e.g., ['succeeded', 'failed']
	limit?: number;
}): Promise<Job[]> {
	const queryParams = new URLSearchParams();
	if (params.source_id) queryParams.set('source_id', params.source_id);
	if (params.status && params.status.length > 0) {
		queryParams.set('status', params.status.join(','));
	}
	if (params.limit) queryParams.set('limit', params.limit.toString());

	const res = await fetch(`${API_BASE}/jobs?${queryParams}`);
	if (!res.ok) throw new Error(`Failed to query jobs: ${res.statusText}`);
	return res.json();
}

export async function cancelJob(jobId: string): Promise<void> {
	const res = await fetch(`${API_BASE}/jobs/${jobId}/cancel`, {
		method: 'POST'
	});
	if (!res.ok) throw new Error(`Failed to cancel job: ${res.statusText}`);
}

// OAuth
export async function initiateOAuth(provider: string, redirectUri?: string, state?: string) {
	const params = new URLSearchParams();
	if (redirectUri) params.set('redirect_uri', redirectUri);
	if (state) params.set('state', state);

	const url = `${API_BASE}/sources/${provider}/authorize${params.toString() ? `?${params}` : ''}`;
	const res = await fetch(url, { method: 'POST' });
	if (!res.ok) throw new Error(`Failed to initiate OAuth: ${res.statusText}`);
	return res.json() as Promise<{ authorization_url: string; state: string }>;
}

export async function handleOAuthCallback(params: {
	code?: string;
	access_token?: string;
	refresh_token?: string;
	expires_in?: number;
	provider: string;
	state?: string;
	workspace_id?: string;
	workspace_name?: string;
	bot_id?: string;
}) {
	const queryParams = new URLSearchParams();
	Object.entries(params).forEach(([key, value]) => {
		if (value !== undefined) {
			queryParams.set(key, value.toString());
		}
	});

	const res = await fetch(`${API_BASE}/sources/callback?${queryParams}`);
	if (!res.ok) throw new Error(`Failed to complete OAuth: ${res.statusText}`);
	return res.json();
}

// Device Pairing
import type {
	InitiatePairingRequest,
	PairingInitResponse,
	CompletePairingRequest,
	PairingCompleteResponse,
	PairingStatus,
	PendingPairing
} from '$lib/types/device-pairing';

/**
 * Initiate device pairing - generates a 6-character pairing code
 * @param deviceType - Type of device (e.g., "ios", "mac")
 * @param name - Display name for the device
 * @returns Pairing code, source ID, and expiration time
 */
export async function initiatePairing(
	deviceType: string,
	name: string
): Promise<PairingInitResponse> {
	const request: InitiatePairingRequest = {
		device_type: deviceType,
		name
	};

	const res = await fetch(`${API_BASE}/devices/pairing/initiate`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});

	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to initiate pairing: ${res.statusText}`);
	}

	return res.json();
}

/**
 * Complete device pairing (typically called by device, not web UI)
 * @param code - 6-character pairing code
 * @param deviceInfo - Device information (ID, name, model, OS version)
 * @returns Device token and available streams
 */
export async function completePairing(
	code: string,
	deviceInfo: CompletePairingRequest['device_info']
): Promise<PairingCompleteResponse> {
	const request: CompletePairingRequest = {
		code,
		device_info: deviceInfo
	};

	const res = await fetch(`${API_BASE}/devices/pairing/complete`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});

	if (!res.ok) {
		const error = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(error.error || `Failed to complete pairing: ${res.statusText}`);
	}

	return res.json();
}

/**
 * Check the status of a device pairing
 * @param sourceId - Source ID from initiatePairing
 * @returns Current pairing status (pending, active, or revoked)
 */
export async function getPairingStatus(sourceId: string): Promise<PairingStatus> {
	const res = await fetch(`${API_BASE}/devices/pairing/${sourceId}`);

	if (!res.ok) {
		throw new Error(`Failed to get pairing status: ${res.statusText}`);
	}

	return res.json();
}

/**
 * List all pending device pairings (not yet completed)
 * @returns Array of pending pairings with codes and expiration times
 */
export async function listPendingPairings(): Promise<{ pairings: PendingPairing[] }> {
	const res = await fetch(`${API_BASE}/devices/pending-pairings`);

	if (!res.ok) {
		throw new Error(`Failed to list pending pairings: ${res.statusText}`);
	}

	return res.json();
}
