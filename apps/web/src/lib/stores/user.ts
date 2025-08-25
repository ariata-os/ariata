import { getContext, setContext } from 'svelte';
import type { users } from '$lib/db/schema';

// Type for user data
export type User = typeof users.$inferSelect;

// Unique key for the context
const USER_CTX = Symbol('user');

/**
 * Set user data in Svelte context
 * Should be called in +layout.svelte
 */
export function setUserContext(user: User) {
	setContext(USER_CTX, user);
}

/**
 * Get user data from Svelte context
 * Can be called in any child component
 */
export function getUserContext(): User {
	const user = getContext<User>(USER_CTX);
	if (!user) {
		throw new Error('User context not found. Make sure to call setUserContext in a parent component.');
	}
	return user;
}

/**
 * Get user context with optional fallback
 * Useful for components that might be used outside the normal layout
 */
export function getUserContextSafe(): User | null {
	try {
		return getContext<User>(USER_CTX);
	} catch {
		return null;
	}
}