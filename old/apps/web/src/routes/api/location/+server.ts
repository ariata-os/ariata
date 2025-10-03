import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db/client';
import { streamIosLocation } from '$lib/db/schema';
import { and, gte, lte, isNotNull } from 'drizzle-orm';
import { parseDate, toZoned } from '@internationalized/date';

export const GET: RequestHandler = async ({ url }) => {
	try {
		const dateParam = url.searchParams.get('date');
		const timezone = url.searchParams.get('timezone') || 'America/Chicago';

		if (!dateParam) {
			return json({ error: 'Date parameter is required' }, { status: 400 });
		}

		// Parse the date and create timezone-aware start/end times
		const calendarDate = parseDate(dateParam);
		const zonedStart = toZoned(calendarDate, timezone);
		const zonedEnd = zonedStart.add({ hours: 23, minutes: 59, seconds: 59, milliseconds: 999 });

		// Convert to standard JS Date objects for the database query
		const startOfDayUTC = zonedStart.toDate();
		const endOfDayUTC = zonedEnd.toDate();

		const rawLocationData = await db
			.select({
				latitude: streamIosLocation.latitude,
				longitude: streamIosLocation.longitude,
				timestamp: streamIosLocation.timestamp,
				horizontalAccuracy: streamIosLocation.horizontalAccuracy,
				speed: streamIosLocation.speed
			})
			.from(streamIosLocation)
			.where(
				and(
					gte(streamIosLocation.timestamp, startOfDayUTC),
					lte(streamIosLocation.timestamp, endOfDayUTC),
					isNotNull(streamIosLocation.latitude),
					isNotNull(streamIosLocation.longitude)
				)
			)
			.orderBy(streamIosLocation.timestamp);

		// Transform to match the expected format: coordinates as [longitude, latitude]
		const coordinateSignals = rawLocationData.map(loc => ({
			coordinates: [loc.longitude, loc.latitude] as [number, number],
			timestamp: loc.timestamp,
			signalValue: loc.speed,
			confidence: loc.horizontalAccuracy
		}));

		return json({ coordinateSignals });
	} catch (error) {
		console.error('Failed to load coordinate signals:', error);
		return json(
			{ error: error instanceof Error ? error.message : 'Failed to load data' },
			{ status: 500 }
		);
	}
};