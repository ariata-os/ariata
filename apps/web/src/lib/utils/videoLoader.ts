// Eagerly import all video files from the assets directory
const videoModules = import.meta.glob(
	'$lib/assets/videos/*.{webm,mp4}',
	{ 
		eager: true
	}
);

// console.log('Video modules loaded:', videoModules);

// Create a map of video filenames to their imported URLs
const videoMap = new Map<string, string>();

// Process imported modules and extract filenames
for (const [path, module] of Object.entries(videoModules)) {
	// Extract filename from path (e.g., '/src/lib/assets/videos/google2.webm' -> 'google2.webm')
	const filename = path.split('/').pop();
	
	// Try to extract the URL from the module
	let url: string | undefined;
	if (typeof module === 'string') {
		url = module;
	} else if (module && typeof module === 'object' && 'default' in module) {
		url = (module as any).default;
	}
	
	if (filename && url) {
		videoMap.set(filename, url);
	}
}

/**
 * Get a video URL by its filename
 * @param filename - The video filename (e.g., 'google2.webm')
 * @returns The imported video URL or null if not found
 */
export function getVideoUrl(filename: string | null | undefined): string | null {
	if (!filename) return null;
	return videoMap.get(filename) || null;
}

/**
 * Get all available video filenames
 * @returns Array of video filenames
 */
export function getAvailableVideos(): string[] {
	return Array.from(videoMap.keys());
}