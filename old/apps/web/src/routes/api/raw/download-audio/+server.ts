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

		const filename = filePath.split('/').pop() || 'audio';
		const ext = filename.toLowerCase().split('.').pop();
		
		// For iOS Opus files, we'll force download since browsers can't play raw Opus
		if (ext === 'opus') {
			return new Response(fileData, {
				headers: {
					'Content-Type': 'application/octet-stream',
					'Content-Disposition': `attachment; filename="${filename}"`,
					'Content-Length': fileData.length.toString()
				}
			});
		}
		
		// For other audio formats, use appropriate content type
		let contentType = 'application/octet-stream';
		switch (ext) {
			case 'wav':
				contentType = 'audio/wav';
				break;
			case 'm4a':
			case 'aac':
				contentType = 'audio/mp4';
				break;
			case 'mp3':
				contentType = 'audio/mpeg';
				break;
			case 'ogg':
			case 'oga':
				contentType = 'audio/ogg';
				break;
			case 'webm':
				contentType = 'audio/webm';
				break;
		}
		
		return new Response(fileData, {
			headers: {
				'Content-Type': contentType,
				'Content-Disposition': `attachment; filename="${filename}"`,
				'Content-Length': fileData.length.toString()
			}
		});
	} catch (err) {
		console.error('Failed to download audio file:', err);
		throw error(500, 'Failed to download audio file');
	}
};