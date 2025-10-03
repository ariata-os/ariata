import { json, type RequestHandler } from '@sveltejs/kit';

// Single-user app - this endpoint returns a static response
export const GET: RequestHandler = async () => {
	return json({ 
		message: 'Single-user app - no user management needed'
	});
};