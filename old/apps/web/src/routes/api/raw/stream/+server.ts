import { error, type RequestHandler } from '@sveltejs/kit';
import { getObjectInfo, getObjectStream } from '$lib/minio';

// Determine content type based on file extension
function getAudioContentType(filename: string): string {
	const ext = filename.toLowerCase().split('.').pop();
	switch (ext) {
		case 'wav':
			return 'audio/wav';
		case 'm4a':
		case 'aac':
			return 'audio/mp4';
		case 'mp3':
			return 'audio/mpeg';
		case 'ogg':
		case 'oga':
			return 'audio/ogg';
		case 'opus':
			// Raw Opus files from iOS - browsers may not support this directly
			// Try audio/opus first, fallback will be handled by browser
			return 'audio/opus';
		case 'caf':
			// Core Audio Format from iOS with Opus codec
			return 'audio/x-caf';
		case 'webm':
			return 'audio/webm';
		case 'flac':
			return 'audio/flac';
		default:
			return 'audio/octet-stream';
	}
}

export const GET: RequestHandler = async ({ url, request }) => {
	const filePath = url.searchParams.get('path');
	
	if (!filePath) {
		throw error(400, 'File path required');
	}

	try {
		// Get file info first
		const fileInfo = await getObjectInfo(filePath);
		if (!fileInfo) {
			throw error(404, 'File not found');
		}

		const fileSize = fileInfo.size;
		const filename = filePath.split('/').pop() || 'audio';
		const contentType = getAudioContentType(filename);

		// Handle Range requests for audio seeking
		const rangeHeader = request.headers.get('range');
		let start = 0;
		let end = fileSize - 1;
		let status = 200;
		let headers: Record<string, string> = {
			'Content-Type': contentType,
			'Accept-Ranges': 'bytes',
			'Content-Length': fileSize.toString(),
			'Cache-Control': 'no-cache',
		};

		if (rangeHeader) {
			const parts = rangeHeader.replace(/bytes=/, '').split('-');
			start = parseInt(parts[0], 10);
			end = parts[1] ? parseInt(parts[1], 10) : fileSize - 1;
			
			if (start >= fileSize || end >= fileSize) {
				throw error(416, 'Requested range not satisfiable');
			}

			status = 206; // Partial Content
			headers = {
				...headers,
				'Content-Range': `bytes ${start}-${end}/${fileSize}`,
				'Content-Length': (end - start + 1).toString(),
			};
		}

		// Get the stream
		const stream = await getObjectStream(filePath, rangeHeader ? { start, end } : undefined);
		if (!stream) {
			throw error(500, 'Failed to get audio stream');
		}

		// Convert Node.js stream to Web stream for Response
		const webStream = new ReadableStream({
			start(controller) {
				stream.on('data', (chunk) => {
					controller.enqueue(chunk);
				});
				stream.on('end', () => {
					controller.close();
				});
				stream.on('error', (err) => {
					controller.error(err);
				});
			},
			cancel() {
				stream.destroy();
			}
		});

		return new Response(webStream, {
			status,
			headers
		});
	} catch (err) {
		console.error('Failed to stream audio file:', err);
		if (err instanceof Error && 'status' in err) {
			throw err;
		}
		throw error(500, 'Failed to stream audio file');
	}
};