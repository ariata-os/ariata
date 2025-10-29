import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import os from 'os';

/**
 * Device Setup Helper - Returns appropriate API endpoint for device pairing
 *
 * Local Dev: Returns Mac's network IP pointing to Rust server (bypasses Vite proxy)
 * Production: Returns production API URL
 */
export const GET: RequestHandler = async ({ request }) => {
	const host = request.headers.get('host') || '';

	// Detect local development
	const isLocalDev = host.includes('localhost') || host.includes('127.0.0.1');

	if (isLocalDev) {
		// Get Mac's network IP address
		const networkIP = getNetworkIP();

		// Return Rust API server address directly (port 8000)
		// iPhone connects directly to Rust server, bypassing Vite proxy
		return json({
			apiEndpoint: `http://${networkIP}:8000`,
			environment: 'development',
			note: 'Connect from iPhone on same WiFi network'
		});
	}

	// Production: use request origin with /api path
	const protocol = host.includes('localhost') ? 'http' : 'https';
	return json({
		apiEndpoint: `${protocol}://${host.split(':')[0]}/api`,
		environment: 'production'
	});
};

/**
 * Get the Mac's local network IP address (e.g., 192.168.1.73)
 */
function getNetworkIP(): string {
	const interfaces = os.networkInterfaces();

	for (const name of Object.keys(interfaces)) {
		const iface = interfaces[name];
		if (!iface) continue;

		for (const addr of iface) {
			// Skip internal (localhost) and non-IPv4 addresses
			if (addr.family === 'IPv4' && !addr.internal) {
				return addr.address;
			}
		}
	}

	// Fallback if no network interface found
	return '192.168.1.1';
}
