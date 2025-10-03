import { error, type RequestHandler } from '@sveltejs/kit';
import { getObjectAsBuffer } from '$lib/minio';

export const GET: RequestHandler = async ({ url }) => {
	// Support both 'file' and 'path' parameters for backward compatibility
	const filePath = url.searchParams.get('file') || url.searchParams.get('path');

	if (!filePath) {
		throw error(400, 'File path required');
	}

	try {
		const fileData = await getObjectAsBuffer(filePath);
		
		if (!fileData) {
			throw error(404, 'File not found');
		}

		let finalData = fileData;
		let filename = filePath.split('/').pop() || 'download';
		
		// Check if file is gzipped by extension or by trying to decompress
		const isGzipped = filePath.endsWith('.gz') || filePath.endsWith('.gzip');
		
		if (isGzipped || filePath.endsWith('.json')) {
			try {
				// Try to decompress - if it fails, use original data
				const { gunzip } = await import('zlib');
				const { promisify } = await import('util');
				const gunzipAsync = promisify(gunzip);
				
				finalData = await gunzipAsync(fileData);
				// Remove .gz extension from filename if present
				if (filename.endsWith('.gz')) {
					filename = filename.slice(0, -3);
				}
			} catch (err) {
				// If decompression fails, assume file is not gzipped
				console.log('File is not gzipped or decompression failed, using original data');
				finalData = fileData;
			}
		}
		
		// Determine content type based on file extension
		let contentType = 'application/octet-stream';
		const lowerFilename = filename.toLowerCase();

		if (lowerFilename.endsWith('.json')) {
			contentType = 'application/json';
		} else if (lowerFilename.endsWith('.html')) {
			contentType = 'text/html';
		} else if (lowerFilename.endsWith('.txt')) {
			contentType = 'text/plain';
		} else if (lowerFilename.endsWith('.csv')) {
			contentType = 'text/csv';
		} else if (lowerFilename.endsWith('.wav')) {
			contentType = 'audio/wav';
		} else if (lowerFilename.endsWith('.mp3')) {
			contentType = 'audio/mpeg';
		} else if (lowerFilename.endsWith('.m4a')) {
			contentType = 'audio/mp4';
		} else if (lowerFilename.endsWith('.aac')) {
			contentType = 'audio/aac';
		} else if (lowerFilename.endsWith('.ogg') || lowerFilename.endsWith('.oga')) {
			contentType = 'audio/ogg';
		} else if (lowerFilename.endsWith('.opus')) {
			contentType = 'audio/opus';
		} else if (lowerFilename.endsWith('.webm')) {
			contentType = 'audio/webm';
		} else if (lowerFilename.endsWith('.flac')) {
			contentType = 'audio/flac';
		}
		
		// Return a response that will trigger a download
		return new Response(finalData, {
			headers: {
				'Content-Type': contentType,
				'Content-Disposition': `attachment; filename="${filename}"`,
				'Content-Length': finalData.length.toString()
			}
		});
	} catch (err) {
		console.error('Failed to download file:', err);
		throw error(500, 'Failed to download file');
	}
};