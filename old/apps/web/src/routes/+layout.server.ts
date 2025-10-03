import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async () => {
	// Single-user app - no user data needed
	return {};
};