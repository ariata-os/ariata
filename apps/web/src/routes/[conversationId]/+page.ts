import type { PageLoad } from './$types';

export const load: PageLoad = async ({ params, fetch }) => {
	const { conversationId } = params;

	// If conversation ID is "new", return empty state
	if (conversationId === 'new') {
		return {
			conversationId: `conv_${Date.now()}_${Math.random().toString(36).substring(7)}`,
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
			throw new Error(`Failed to load conversation: ${response.statusText}`);
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
