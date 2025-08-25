import { error, type RequestHandler } from '@sveltejs/kit';
import { getObjectAsBuffer } from '$lib/minio';

export const GET: RequestHandler = async ({ url }) => {
	const filePath = url.searchParams.get('path');
	
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
		if (filename.endsWith('.json')) {
			contentType = 'application/json';
		} else if (filename.endsWith('.html')) {
			contentType = 'text/html';
		} else if (filename.endsWith('.txt')) {
			contentType = 'text/plain';
		} else if (filename.endsWith('.csv')) {
			contentType = 'text/csv';
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