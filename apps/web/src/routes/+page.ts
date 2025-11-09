import type { PageLoad } from './$types';

export const load: PageLoad = async () => {
	// Root path always creates a new conversation
	return {
		conversationId: crypto.randomUUID(),
		messages: [],
		isNew: true
	};
};
