/**
 * Get a video URL by its filename from the static directory
 * @param filename - The video filename (e.g., 'google2.webm')
 * @returns The video URL or null if not provided
 */
export function getVideoUrl(filename: string | null | undefined): string | null {
	if (!filename) return null;
	// Videos are now served from the static directory
	return `/videos/${filename}`;
}

/**
 * Get all available video filenames
 * @returns Array of video filenames
 */
export function getAvailableVideos(): string[] {
	// Return the known video filenames
	return [
		'google2.webm',
		'ios.webm',
		'mac2.webm',
		'notion.webm',
		'strava.webm'
	];
}