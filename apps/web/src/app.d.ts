// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
import type { ApiClient } from '$lib/server/apiClient';

declare global {
	namespace App {
		// interface Error {}
		interface Locals {
			apiClient: ApiClient;
		}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
