import { generateText } from 'ai';
import type { ChatMessage } from '$lib/server/schema';

/**
 * Configuration for subject generation
 */
const CONFIG = {
	model: 'google/gemini-2.5-flash-lite',
	maxRetries: 3,
	baseDelay: 1000, // 1 second base delay for exponential backoff
	maxSubjectLength: 50,
	timeout: 10000, // 10 seconds per attempt
};

/**
 * Sleep for a specified number of milliseconds
 */
function sleep(ms: number): Promise<void> {
	return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * Create a fallback subject by truncating the user message
 */
function createFallbackSubject(userMessage: string): string {
	// Remove extra whitespace and truncate
	const cleaned = userMessage.replace(/\s+/g, ' ').trim();

	if (cleaned.length <= CONFIG.maxSubjectLength) {
		return cleaned;
	}

	// Try to cut at a word boundary
	const truncated = cleaned.substring(0, CONFIG.maxSubjectLength);
	const lastSpace = truncated.lastIndexOf(' ');

	if (lastSpace > CONFIG.maxSubjectLength * 0.6) {
		return truncated.substring(0, lastSpace) + '...';
	}

	return truncated + '...';
}

/**
 * Generate a subject for a chat exchange using AI
 */
async function generateWithAI(
	userMessage: string,
	assistantMessage: string,
	firstUserMessage?: string
): Promise<string> {
	const contextInfo = firstUserMessage
		? `Context: The conversation started with the user saying: "${firstUserMessage}"\n\n`
		: '';

	const prompt = `${contextInfo}Generate a very brief, descriptive subject line (3-6 words) for this exchange between a user and an AI assistant. The subject should capture the essence of what the user is asking about or discussing.

User message: ${userMessage}

Assistant response: ${assistantMessage}

Requirements:
- Maximum 6 words
- Be specific and descriptive
- Focus on the user's intent or topic
- Do not include generic words like "question", "query", "request"
- Do not include quotation marks

Subject:`;

	const { text } = await generateText({
		model: CONFIG.model,
		prompt,
		maxRetries: 0, // We handle retries ourselves
		abortSignal: AbortSignal.timeout(CONFIG.timeout),
	});

	// Clean up the generated text
	const cleaned = text
		.trim()
		.replace(/^["']|["']$/g, '') // Remove quotes
		.replace(/\.$/, ''); // Remove trailing period

	// Validate the result isn't too long
	if (cleaned.length > CONFIG.maxSubjectLength) {
		return createFallbackSubject(userMessage);
	}

	return cleaned;
}

/**
 * Generate a subject for a chat exchange with retry logic and fallback
 *
 * @param messages - The chat messages array
 * @param exchangeIndex - The index of the user message to generate a subject for
 * @returns A subject string (always succeeds via fallback if needed)
 */
export async function generateSubject(
	messages: ChatMessage[],
	exchangeIndex: number
): Promise<string> {
	// Find the user message at the specified index
	const userMessages = messages.filter(m => m.role === 'user');

	if (exchangeIndex >= userMessages.length) {
		throw new Error(`Exchange index ${exchangeIndex} out of bounds`);
	}

	const targetUserMessage = userMessages[exchangeIndex];
	const userMessageIndex = messages.indexOf(targetUserMessage);

	// Find the corresponding assistant response
	const assistantResponse = messages
		.slice(userMessageIndex + 1)
		.find(m => m.role === 'assistant');

	if (!assistantResponse) {
		// No assistant response yet, just use fallback
		return createFallbackSubject(targetUserMessage.content);
	}

	// Get the first user message for context (if different from current)
	const firstUserMessage = exchangeIndex > 0 ? userMessages[0].content : undefined;

	// Try AI generation with retries
	for (let attempt = 0; attempt < CONFIG.maxRetries; attempt++) {
		try {
			console.log(`[SubjectGenerator] Attempt ${attempt + 1}/${CONFIG.maxRetries} using ${CONFIG.model}`);

			const subject = await generateWithAI(
				targetUserMessage.content,
				assistantResponse.content,
				firstUserMessage
			);

			console.log(`[SubjectGenerator] Successfully generated subject: "${subject}"`);
			return subject;

		} catch (error) {
			const isLastAttempt = attempt === CONFIG.maxRetries - 1;

			console.error(
				`[SubjectGenerator] Attempt ${attempt + 1} failed:`,
				error instanceof Error ? error.message : 'Unknown error'
			);

			if (isLastAttempt) {
				console.log('[SubjectGenerator] All attempts failed, using fallback');
				break;
			}

			// Exponential backoff: 1s, 2s, 4s
			const delay = CONFIG.baseDelay * Math.pow(2, attempt);
			console.log(`[SubjectGenerator] Waiting ${delay}ms before retry...`);
			await sleep(delay);
		}
	}

	// Fallback: always return something
	const fallback = createFallbackSubject(targetUserMessage.content);
	console.log(`[SubjectGenerator] Using fallback subject: "${fallback}"`);
	return fallback;
}

/**
 * Check if a subject already exists for a given exchange
 */
export function hasSubject(messages: ChatMessage[], exchangeIndex: number): boolean {
	const userMessages = messages.filter(m => m.role === 'user');
	return exchangeIndex < userMessages.length && !!userMessages[exchangeIndex].subject;
}

/**
 * Update the subject in a messages array (returns new array)
 */
export function updateSubjectInMessages(
	messages: ChatMessage[],
	exchangeIndex: number,
	subject: string
): ChatMessage[] {
	const userMessages = messages.filter(m => m.role === 'user');

	if (exchangeIndex >= userMessages.length) {
		throw new Error(`Exchange index ${exchangeIndex} out of bounds`);
	}

	const targetUserMessage = userMessages[exchangeIndex];
	const messageIndex = messages.indexOf(targetUserMessage);

	// Create a new array with the updated message
	const updatedMessages = [...messages];
	updatedMessages[messageIndex] = {
		...targetUserMessage,
		subject
	};

	return updatedMessages;
}