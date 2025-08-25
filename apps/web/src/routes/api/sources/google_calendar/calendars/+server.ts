import { type RequestHandler } from '@sveltejs/kit';
import { db } from '$lib/db/client';
import { sources } from '$lib/db/schema';
import { eq, and } from 'drizzle-orm';

export const GET: RequestHandler = async ({ url }) => {
	try {
		const connectionId = url.searchParams.get('connection_id');

		if (!connectionId) {
			return new Response(JSON.stringify({ error: 'Connection ID is required' }), {
				status: 400,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		// Get the Google Calendar source for this user
		const [source] = await db
			.select()
			.from(sourceConfigs)
			.where(and(
				eq(sourceConfigs.name, 'google_calendar'),
				eq(sourceConfigs.userId, userId)
			))
			.limit(1);

		if (!source || !source.oauthAccessToken) {
			return new Response(JSON.stringify({ error: 'No credentials found for Google Calendar' }), {
				status: 404,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		// Check if token is expired
		if (source.oauthExpiresAt && new Date() >= source.oauthExpiresAt) {
			return new Response(JSON.stringify({ error: 'Access token expired. Please re-authenticate.' }), {
				status: 401,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		// Make request to Google Calendar API
		const response = await fetch('https://www.googleapis.com/calendar/v3/users/me/calendarList', {
			headers: {
				'Authorization': `Bearer ${source.oauthAccessToken}`,
				'Accept': 'application/json'
			}
		});

		if (!response.ok) {
			if (response.status === 401) {
				return new Response(JSON.stringify({ error: 'Invalid or expired access token. Please re-authenticate.' }), {
					status: 401,
					headers: { 'Content-Type': 'application/json' }
				});
			}
			
			const errorText = await response.text();
			console.error('Google Calendar API error:', response.status, errorText);
			return new Response(JSON.stringify({ error: 'Failed to fetch calendars from Google' }), {
				status: response.status,
				headers: { 'Content-Type': 'application/json' }
			});
		}

		const data = await response.json();
		const calendars = data.items || [];

		// Transform calendars to the format expected by the frontend
		const transformedCalendars = calendars.map((calendar: any) => ({
			value: calendar.id,
			label: calendar.summary,
			description: calendar.description || '',
			primary: calendar.primary || false,
			accessRole: calendar.accessRole,
			colorId: calendar.colorId
		}));

		return new Response(JSON.stringify({
			options: transformedCalendars,
			total: transformedCalendars.length
		}), {
			status: 200,
			headers: { 'Content-Type': 'application/json' }
		});

	} catch (error) {
		console.error('Error fetching Google Calendar calendars:', error);
		return new Response(JSON.stringify({ error: 'Internal server error' }), {
			status: 500,
			headers: { 'Content-Type': 'application/json' }
		});
	}
};