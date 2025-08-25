import type { PageServerLoad, Actions } from './$types';
import { error } from '@sveltejs/kit';
import { minioClient, BUCKET_NAME, getObjectAsBuffer, getStreamKeys, getStreamData } from '$lib/minio';
import { db } from '$lib/db/client';
import { streamConfigs } from '$lib/db/schema';
import { eq } from 'drizzle-orm';

interface DirectoryItem {
	name: string;
	type: 'file' | 'directory';
	size?: number;
	lastModified?: Date;
	path: string;
}

async function listDirectory(prefix: string = ''): Promise<DirectoryItem[]> {
	try {
		const items: DirectoryItem[] = [];
		
		// Use listObjectsV2 with delimiter for proper directory listing
		const stream = minioClient.listObjectsV2(BUCKET_NAME, prefix, false, '/');
		
		return new Promise((resolve) => {
			// Add timeout to prevent hanging
			const timeout = setTimeout(() => {
				stream.destroy();
				console.error('MinIO listing timeout');
				resolve([]);
			}, 5000); // 5 second timeout
			
			stream.on('data', (obj) => {
				if (obj.prefix) {
					// This is a directory
					const dirName = obj.prefix.substring(prefix.length).replace(/\/$/, '');
					if (dirName) {
						items.push({
							name: dirName,
							type: 'directory',
							path: obj.prefix.slice(0, -1) // Remove trailing slash
						});
					}
				} else if (obj.name) {
					// This is a file
					const fileName = obj.name.substring(prefix.length);
					if (fileName && !fileName.includes('/')) {
						items.push({
							name: fileName,
							type: 'file',
							size: obj.size || 0,
							lastModified: obj.lastModified || new Date(),
							path: obj.name
						});
					}
				}
			});
			
			stream.on('end', () => {
				clearTimeout(timeout);
				// Sort: directories first, then files, both alphabetically
				const sorted = items.sort((a, b) => {
					if (a.type !== b.type) {
						return a.type === 'directory' ? -1 : 1;
					}
					return a.name.localeCompare(b.name);
				});
				resolve(sorted);
			});
			
			stream.on('error', (err) => {
				clearTimeout(timeout);
				console.error('MinIO stream error:', err);
				resolve([]);
			});
		});
	} catch (err) {
		console.error('Failed to list MinIO directory:', err);
		return [];
	}
}

export const load: PageServerLoad = async ({ params }) => {
	// Handle SvelteKit's rest parameter - it can be a string or undefined
	// For root /data/raw, params.path will be undefined
	const pathSegments = params.path ? params.path.split('/').filter(p => p) : [];
	const path = pathSegments.length > 0 ? pathSegments.join('/') + '/' : '';
	
	// Check if the first segment is a stream name
	let isStream = false;
	let streamData = null;
	let streamKeys = null;
	let sampleData = null;
	let streamStats = null;
	
	if (pathSegments.length >= 1) {
		const potentialStreamName = pathSegments[0];
		
		// Check if this is a stream in the database
		const [stream] = await db
			.select()
			.from(streamConfigs)
			.where(eq(streamConfigs.streamName, potentialStreamName))
			.limit(1);
		
		if (stream && pathSegments.length === 1) {
			// We're at the stream root level - show enhanced stream view
			isStream = true;
			streamData = stream;
			
			// Get recent stream keys from MinIO (last 10 entries)
			streamKeys = await getStreamKeys(potentialStreamName, 10);
			
			// Get sample data from the most recent entry
			if (streamKeys.length > 0) {
				try {
					const mostRecentKey = streamKeys[0];
					sampleData = await getStreamData(mostRecentKey);
				} catch (err) {
					console.error('Failed to fetch stream sample data:', err);
				}
			}
			
			streamStats = {
				totalEntries: streamKeys.length,
				dataSize: streamKeys.reduce((acc, key) => acc + (key.size || 0), 0),
				lastModified: streamKeys[0]?.lastModified || null
			};
		}
	}
	
	try {
		const items = await listDirectory(path);
		
		// Bucket stats disabled for performance
		const bucketStats = { totalSize: 0, totalObjects: 0 };
		
		// Build breadcrumb navigation
		const breadcrumbs = [
			{ name: 'raw', path: '/data/raw' }
		];
		
		let currentPath = '';
		for (const part of pathSegments) {
			currentPath += part + '/';
			breadcrumbs.push({
				name: part,
				path: `/data/raw/${currentPath.slice(0, -1)}`
			});
		}

		return {
			items,
			currentPath: path,
			breadcrumbs,
			isRoot: pathSegments.length === 0,
			bucketStats,
			isStream,
			stream: streamData,
			streamKeys,
			sampleData,
			stats: streamStats
		};
	} catch (err) {
		console.error('Failed to load directory:', err);
		throw error(500, 'Failed to load directory');
	}
};

export const actions: Actions = {
	view: async ({ request }) => {
		const data = await request.formData();
		const filePath = data.get('file') as string;
		
		if (!filePath) {
			throw error(400, 'File path required');
		}

		try {
			const fileData = await getObjectAsBuffer(filePath);
			
			if (!fileData) {
				throw error(404, 'File not found');
			}

			let jsonContent: string;
			
			// Check if file is gzipped and try to decompress
			const isGzipped = filePath.endsWith('.gz') || filePath.endsWith('.gzip');
			
			if (isGzipped || filePath.endsWith('.json')) {
				try {
					// Try to decompress
					const { gunzip } = await import('zlib');
					const { promisify } = await import('util');
					const gunzipAsync = promisify(gunzip);
					
					const decompressed = await gunzipAsync(fileData);
					jsonContent = decompressed.toString();
				} catch (err) {
					// If decompression fails, assume file is not gzipped
					console.log('File is not gzipped or decompression failed, using original data');
					jsonContent = fileData.toString();
				}
			} else {
				// For non-JSON files, just convert to string
				jsonContent = fileData.toString();
			}
			
			// Parse and format JSON if possible
			let formattedContent;
			try {
				const parsed = JSON.parse(jsonContent);
				formattedContent = JSON.stringify(parsed, null, 2);
			} catch {
				formattedContent = jsonContent; // If not valid JSON, show raw content
			}
			
			return {
				type: 'view',
				content: formattedContent,
				filename: filePath.split('/').pop(),
				path: filePath
			};
		} catch (err) {
			console.error('Failed to view file:', err);
			throw error(500, 'Failed to view file');
		}
	},

	play: async ({ request }) => {
		const data = await request.formData();
		const filePath = data.get('file') as string;
		
		if (!filePath) {
			throw error(400, 'File path required');
		}

		// We don't need to load the actual audio data here
		// Just return the path and filename for the audio player
		return {
			type: 'play',
			path: filePath,
			filename: filePath.split('/').pop() || 'audio'
		};
	}
};