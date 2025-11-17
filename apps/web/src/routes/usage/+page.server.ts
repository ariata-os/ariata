import type { PageServerLoad } from './$types';
import { getUsageStats } from '$lib/server/rate-limit';

export const load: PageServerLoad = async () => {
	try {
		// Get usage stats for chat endpoint
		const chatStats = await getUsageStats('chat');

		// Calculate percentage usage
		const dailyRequestsPercentage = Math.round(
			(chatStats.dailyRequests / chatStats.limits.chatRequestsPerDay) * 100
		);
		const dailyTokensPercentage = Math.round(
			(chatStats.dailyTokens / chatStats.limits.chatTokensPerDay) * 100
		);

		return {
			usage: {
				daily: {
					requests: chatStats.dailyRequests,
					requestsLimit: chatStats.limits.chatRequestsPerDay,
					requestsPercentage: dailyRequestsPercentage,
					tokens: chatStats.dailyTokens,
					tokensLimit: chatStats.limits.chatTokensPerDay,
					tokensPercentage: dailyTokensPercentage,
					cost: chatStats.dailyCost
				},
				limits: chatStats.limits
			}
		};
	} catch (error) {
		console.error('Failed to load usage stats:', error);
		// Return default values if there's an error
		return {
			usage: {
				daily: {
					requests: 0,
					requestsLimit: 1000,
					requestsPercentage: 0,
					tokens: 0,
					tokensLimit: 500000,
					tokensPercentage: 0,
					cost: 0
				},
				limits: {
					chatRequestsPerDay: 1000,
					chatTokensPerDay: 500000,
					backgroundJobsPerDay: 100
				}
			}
		};
	}
};
