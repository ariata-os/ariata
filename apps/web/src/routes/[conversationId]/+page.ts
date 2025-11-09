import type { PageLoad } from './$types';

export const load: PageLoad = async ({ params, fetch }) => {
	const { conversationId } = params;

	// If conversation ID is "new", return empty state
	if (conversationId === 'new') {
		return {
			conversationId: crypto.randomUUID(),
			messages: [],
			isNew: true
		};
	}

	// Otherwise, load existing conversation
	try {
		const response = await fetch(`/api/sessions/${conversationId}`);

		if (!response.ok) {
			// If conversation doesn't exist, create a new one
			if (response.status === 404) {
				return {
					conversationId,
					messages: [],
					isNew: true
				};
			}

			// Try to get error details from response
			let errorMessage = response.statusText;
			try {
				const errorData = await response.json();
				errorMessage = errorData.error || errorData.details || errorMessage;
			} catch {
				// If JSON parsing fails, use statusText
			}

			throw new Error(`Failed to load conversation: ${errorMessage}`);
		}

		// Check if response has content before parsing
		const contentType = response.headers.get('content-type');
		if (!contentType || !contentType.includes('application/json')) {
			console.error('Invalid content-type:', contentType);
			throw new Error('Invalid response format from server');
		}

		const data = await response.json();

		return {
			conversationId,
			conversation: data.conversation,
			messages: data.messages || [],
			isNew: false
		};
	} catch (error) {
		console.error('Error loading conversation:', error);
		// Return empty state on error
		return {
			conversationId,
			messages: [],
			isNew: true,
			error: error instanceof Error ? error.message : 'Failed to load conversation'
		};
	}
};
