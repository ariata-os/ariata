/**
 * Derive the signal type from the signal name
 * e.g., "apple_ios_speed" -> "speed", "google_api_calendar" -> "calendar"
 */
export function deriveSignalType(signalName: string): string {
	// Remove source prefix to get the type
	const parts = signalName.split('_');
	
	// Handle different patterns
	if (signalName.startsWith('google_api_')) {
		// google_api_calendar -> calendar
		return parts.slice(2).join('_');
	} else if (signalName.startsWith('apple_ios_') || signalName.startsWith('apple_mac_')) {
		// apple_ios_speed -> speed, apple_mac_apps -> apps
		return parts.slice(2).join('_');
	} else if (parts.length >= 2) {
		// Fallback for other patterns
		return parts.slice(1).join('_');
	}
	
	// Fallback to the full name if pattern doesn't match
	return signalName;
}

/**
 * Get a human-readable signal type
 * e.g., "speed" -> "Speed", "calendar_events" -> "Calendar Events"
 */
export function formatSignalType(signalType: string): string {
	return signalType
		.split('_')
		.map(word => word.charAt(0).toUpperCase() + word.slice(1))
		.join(' ');
}