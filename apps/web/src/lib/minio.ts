import * as Minio from 'minio';

// Parse MinIO endpoint configuration
function getMinioConfig() {
	const endpoint = process.env.MINIO_ENDPOINT || 'minio:9000';
	const [host, port] = endpoint.replace(/^https?:\/\//, '').split(':');
	
	return {
		endPoint: host,
		port: port ? parseInt(port) : 9000,
		useSSL: process.env.MINIO_USE_SSL?.toLowerCase() === 'true',
		accessKey: process.env.MINIO_ACCESS_KEY || process.env.MINIO_ROOT_USER || 'minioadmin',
		secretKey: process.env.MINIO_SECRET_KEY || process.env.MINIO_ROOT_PASSWORD || 'minioadmin',
	};
}

// Create a singleton MinIO client instance
export const minioClient = new Minio.Client(getMinioConfig());

// Export the bucket name as a constant
export const BUCKET_NAME = 'ariata';

// Helper function to get an object as a buffer
export async function getObjectAsBuffer(key: string): Promise<Buffer | null> {
	try {
		const stream = await minioClient.getObject(BUCKET_NAME, key);
		const chunks: Buffer[] = [];
		
		return new Promise((resolve) => {
			stream.on('data', (chunk) => chunks.push(chunk));
			stream.on('end', () => resolve(Buffer.concat(chunks)));
			stream.on('error', () => resolve(null));
		});
	} catch (err) {
		console.error('Failed to get MinIO object:', err);
		return null;
	}
}

// Helper function to get object info
export async function getObjectInfo(key: string) {
	try {
		return await minioClient.statObject(BUCKET_NAME, key);
	} catch (err) {
		console.error('Failed to get MinIO object info:', err);
		return null;
	}
}

// Helper function to get object stream with optional range
export async function getObjectStream(key: string, range?: { start: number; end: number }) {
	try {
		if (range) {
			return await minioClient.getPartialObject(BUCKET_NAME, key, range.start, range.end - range.start + 1);
		}
		return await minioClient.getObject(BUCKET_NAME, key);
	} catch (err) {
		console.error('Failed to get MinIO object stream:', err);
		return null;
	}
}

// Get list of stream keys with metadata
export async function getStreamKeys(streamName: string, limit: number = 10): Promise<Array<{
	key: string;
	size: number;
	lastModified: Date;
}>> {
	const prefix = `${streamName}/`;
	const objects: Array<{ key: string; size: number; lastModified: Date }> = [];
	
	return new Promise((resolve, reject) => {
		const stream = minioClient.listObjectsV2(BUCKET_NAME, prefix, true);
		
		stream.on('data', (obj) => {
			if (objects.length < limit && obj.name) {
				objects.push({
					key: obj.name,
					size: obj.size || 0,
					lastModified: obj.lastModified || new Date()
				});
			}
		});
		
		stream.on('error', (err) => {
			console.error('Error listing stream objects:', err);
			reject(err);
		});
		
		stream.on('end', () => {
			// Sort by last modified date (newest first)
			objects.sort((a, b) => b.lastModified.getTime() - a.lastModified.getTime());
			resolve(objects.slice(0, limit));
		});
	});
}

// Get stream data from MinIO
export async function getStreamData(key: string): Promise<any> {
	const buffer = await getObjectAsBuffer(key);
	if (!buffer) return null;
	
	try {
		// Try to decompress if gzipped
		if (key.endsWith('.gz') || buffer[0] === 0x1f && buffer[1] === 0x8b) {
			const { gunzipSync } = await import('zlib');
			const decompressed = gunzipSync(buffer);
			return JSON.parse(decompressed.toString());
		}
		
		// Otherwise parse as JSON
		return JSON.parse(buffer.toString());
	} catch (err) {
		console.error('Failed to parse stream data:', err);
		return null;
	}
}

// Helper function to store stream data
export async function storeStreamData(
	streamName: string,
	data: any,
	metadata?: Record<string, string>
): Promise<{ key: string; sizeBytes: number }> {
	const now = new Date();
	const datePath = `${now.getUTCFullYear()}/${String(now.getUTCMonth() + 1).padStart(2, '0')}/${String(now.getUTCDate()).padStart(2, '0')}`;
	const batchId = crypto.randomUUID();
	const key = `streams/${streamName}/${datePath}/${batchId}.json`;
	
	const jsonData = JSON.stringify(data);
	const sizeBytes = jsonData.length;
	
	await minioClient.putObject(
		BUCKET_NAME,
		key,
		jsonData,
		sizeBytes,
		{
			'Content-Type': 'application/json',
			...metadata
		}
	);
	
	return { key, sizeBytes };
}