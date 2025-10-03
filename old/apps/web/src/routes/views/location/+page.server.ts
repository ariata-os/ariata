// src/routes/location/+page.server.ts

import type { PageServerLoad } from './$types';
import { now, parseDate } from '@internationalized/date';

export const load: PageServerLoad = async ({ url }) => {
	// Default timezone for single-user app
	const userTimezone = 'America/Chicago';

	// Get date from query params or use today
	const dateParam = url.searchParams.get('date');

	let selectedDate: string;
	if (dateParam) {
		selectedDate = dateParam;
	} else {
		// Get today in user's timezone
		const todayZoned = now(userTimezone);
		const year = todayZoned.year;
		const month = String(todayZoned.month).padStart(2, '0');
		const day = String(todayZoned.day).padStart(2, '0');
		selectedDate = `${year}-${month}-${day}`;
	}

	return {
		selectedDate,
		userTimezone
	};
};