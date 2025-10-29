/**
 * Device Pairing Types
 *
 * Type definitions for device pairing flow matching Rust backend API
 */

// Device information structure
export interface DeviceInfo {
	device_id: string;
	device_name: string;
	device_model: string;
	os_version: string;
	app_version?: string;
}

// Response when initiating device pairing
export interface PairingInitResponse {
	source_id: string;
	code: string; // 6-char alphanumeric code
	expires_at: string; // ISO 8601 timestamp
}

// Response when completing device pairing
export interface PairingCompleteResponse {
	source_id: string;
	device_token: string; // Base64 encoded token
	available_streams: Stream[];
}

// Stream information
export interface Stream {
	stream_name: string;
	display_name: string;
	description: string;
	is_enabled: boolean;
	supports_incremental: boolean;
	default_cron_schedule: string | null;
}

// Pairing status variants
export type PairingStatusType = 'pending' | 'active' | 'revoked';

export interface PairingStatusPending {
	status: 'pending';
}

export interface PairingStatusActive {
	status: 'active';
	device_info: DeviceInfo;
}

export interface PairingStatusRevoked {
	status: 'revoked';
}

export type PairingStatus = PairingStatusPending | PairingStatusActive | PairingStatusRevoked;

// Pending pairing information
export interface PendingPairing {
	source_id: string;
	name: string;
	device_type: string;
	code: string;
	expires_at: string; // ISO 8601 timestamp
	created_at: string; // ISO 8601 timestamp
}

// Request types
export interface InitiatePairingRequest {
	device_type: string; // e.g., "ios", "mac"
	name: string; // Device name
}

export interface CompletePairingRequest {
	code: string;
	device_info: DeviceInfo;
}
