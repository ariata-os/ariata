import type { PageLoad } from './$types';

export const load: PageLoad = async () => {
	// Root path always creates a new conversation
	return {
		conversationId: `conv_${Date.now()}_${Math.random().toString(36).substring(7)}`,
		messages: [],
		isNew: true
	};
};
