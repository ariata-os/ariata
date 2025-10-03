import { HfInference } from '@huggingface/inference';
import { env } from '$env/dynamic/private';
import type { RequestHandler } from './$types';

const hf = new HfInference(env.HUGGINGFACE_API_KEY || env.HF_TOKEN);

export const POST: RequestHandler = async ({ request }) => {
	try {
		const { messages } = await request.json();

		// Use openai/gpt-oss-120b via Cerebras through HuggingFace
		const stream = hf.chatCompletionStream({
			model: 'openai/gpt-oss-120b',
			messages: messages,
			max_tokens: 500,
			temperature: 0.7
		});

		// Create a ReadableStream from the HuggingFace response
		const encoder = new TextEncoder();
		const readableStream = new ReadableStream({
			async start(controller) {
				try {
					let chunkCount = 0;
					for await (const chunk of stream) {
						const content = chunk.choices[0]?.delta?.content;
						if (content) {
							chunkCount++;
							console.log(`Chunk ${chunkCount}:`, content);
							// Format as AI SDK data stream protocol
							const escaped = content.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n');
							const formatted = `0:"${escaped}"\n`;
							controller.enqueue(encoder.encode(formatted));
						}
					}
					console.log(`Stream complete. Total chunks: ${chunkCount}`);
					controller.close();
				} catch (error) {
					console.error('Stream error:', error);
					controller.error(error);
				}
			}
		});

		return new Response(readableStream, {
			headers: {
				'Content-Type': 'text/plain; charset=utf-8',
				'X-Content-Type-Options': 'nosniff'
			}
		});
	} catch (error) {
		console.error('Chat API error:', error);
		console.error('Error details:', error instanceof Error ? error.message : JSON.stringify(error));
		return new Response(
			JSON.stringify({
				error: 'Failed to process chat request',
				details: error instanceof Error ? error.message : 'Unknown error'
			}),
			{
				status: 500,
				headers: { 'Content-Type': 'application/json' }
			}
		);
	}
};
