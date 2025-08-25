import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db/client';
import { signals } from '$lib/db/schema';
import { eq, and, gte, lte, isNotNull } from 'drizzle-orm';
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

		const rawCoordinateSignals = await db
			.select({
				coordinates: signals.coordinates,
				timestamp: signals.timestamp,
				signalValue: signals.signalValue,
				confidence: signals.confidence
			})
			.from(signals)
			.where(
				and(
					eq(signals.signalName, 'ios_coordinates'),
					gte(signals.timestamp, startOfDayUTC),
					lte(signals.timestamp, endOfDayUTC),
					isNotNull(signals.coordinates)
				)
			)
			.orderBy(signals.timestamp);

		// Filter out records without valid coordinates and ensure proper format
		const coordinateSignals = rawCoordinateSignals
			.filter(signal => signal.coordinates !== null)
			.map(signal => ({
				...signal,
				// Ensure coordinates are in [longitude, latitude] format
				coordinates: signal.coordinates as [number, number]
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