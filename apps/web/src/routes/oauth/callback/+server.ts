import { type RequestHandler } from '@sveltejs/kit';
import { redirect } from '@sveltejs/kit';
import { db } from '$lib/db/client';
import { sourceConfigs, sources } from '$lib/db/schema';
import { eq } from 'drizzle-orm';

export const GET: RequestHandler = async ({ url, cookies }) => {
	console.log('OAuth callback received with params:', {
		hasAccessToken: !!url.searchParams.get('access_token'),
		hasRefreshToken: !!url.searchParams.get('refresh_token'),
		provider: url.searchParams.get('provider'),
		state: url.searchParams.get('state'),
		error: url.searchParams.get('error'),
		allParams: Array.from(url.searchParams.keys()),
		cookies: cookies.getAll().map(c => c.name)
	});

	// Get OAuth callback parameters from auth proxy
	const accessToken = url.searchParams.get('access_token');
	const refreshToken = url.searchParams.get('refresh_token');
	const expiresIn = url.searchParams.get('expires_in');
	const provider = url.searchParams.get('provider');
	const errorParam = url.searchParams.get('error');

	// Parse state parameter which contains both return path and source ID
	const state = url.searchParams.get('state');
	let returnPath = '/data/sources';
	let pendingSourceId: string | null = null;
	
	if (state) {
		// State format: "returnPath|sourceId" or just "returnPath"
		const stateParts = state.split('|');
		returnPath = stateParts[0] || '/data/sources';
		pendingSourceId = stateParts[1] || null;
	}
	
	// Ensure the return path is valid and safe
	if (!returnPath.startsWith('/')) {
		returnPath = '/data/sources';
	}
	
	console.log(`OAuth callback - state: ${state}, returnPath: ${returnPath}, sourceId: ${pendingSourceId}`);

	// Handle OAuth errors
	if (errorParam) {
		console.error('OAuth error from auth proxy:', errorParam);
		const errorDescription = url.searchParams.get('error_description') || errorParam;
		return redirect(302, returnPath + '?oauth_error=' + encodeURIComponent(errorDescription));
	}

	// Validate required parameters
	if (!accessToken || !provider) {
		console.error('Missing OAuth tokens:', {
			accessToken: !!accessToken,
			provider,
			url: url.toString()
		});
		return redirect(302, returnPath + '?oauth_error=Missing+authentication+tokens.+Check+auth+proxy+configuration.');
	}

	try {
		// Map provider to source name
		const sourceName = provider; // Now we use consistent naming

		// Check if source exists and get its configuration
		const [sourceConfig] = await db
			.select()
			.from(sourceConfigs)
			.where(eq(sourceConfigs.name, sourceName))
			.limit(1);

		if (!sourceConfig) {
			console.error('Source not found:', sourceName);
			return redirect(302, returnPath + '?oauth_error=' + encodeURIComponent(`Source '${sourceName}' not found`));
		}

		// Get required scopes from source configuration
		const requiredScopes = sourceConfig.oauthConfig?.requiredScopes || [];
		console.log(`Using scopes from configuration for ${sourceName}:`, requiredScopes);

		// In the new stream-based architecture, we don't auto-activate signals
		// We just create the authenticated source instance and let the user activate streams

		// Create a source instance with OAuth credentials
		const expiresAt = expiresIn
			? new Date(Date.now() + parseInt(expiresIn) * 1000)
			: null;

		let sourceInstance;
		
		if (pendingSourceId) {
			// Update the specific pending source with OAuth tokens
			const [existingSource] = await db
				.select()
				.from(sources)
				.where(eq(sources.id, pendingSourceId))
				.limit(1);
			
			if (existingSource && existingSource.status === 'pending') {
				// Update the pending source with OAuth credentials
				const [updatedSource] = await db
					.update(sources)
					.set({
						status: 'authenticated',
						oauthAccessToken: accessToken,
						oauthRefreshToken: refreshToken || null,
						oauthExpiresAt: expiresAt,
						scopes: requiredScopes,
						sourceMetadata: {
							...((existingSource.sourceMetadata as any) || {}),
							connectedAt: new Date().toISOString(),
							provider: provider,
							isPending: false
						},
						updatedAt: new Date()
					})
					.where(eq(sources.id, pendingSourceId))
					.returning();
				sourceInstance = updatedSource;
				console.log(`OAuth connection successful: updated pending source ${updatedSource.id} for ${sourceName}`);
			} else {
				console.warn(`Pending source ${pendingSourceId} not found or not in pending status`);
				// Fall back to creating a new source
				const instanceName = `${sourceConfig.displayName || sourceConfig.name} Account`;
				const [newSource] = await db
					.insert(sources)
					.values({
						sourceName: sourceName,
						instanceName: instanceName,
						status: 'authenticated',
						oauthAccessToken: accessToken,
						oauthRefreshToken: refreshToken || null,
						oauthExpiresAt: expiresAt,
						scopes: requiredScopes,
						sourceMetadata: {
							connectedAt: new Date().toISOString(),
							provider: provider
						}
					})
					.returning();
				sourceInstance = newSource;
				console.log(`OAuth connection successful: created new source ${newSource.id} for ${sourceName}`);
			}
		} else {
			// No pending source ID provided - create a new source
			// This maintains backward compatibility
			const instanceName = `${sourceConfig.displayName || sourceConfig.name} Account`;
			const [newSource] = await db
				.insert(sources)
				.values({
					sourceName: sourceName,
					instanceName: instanceName,
					status: 'authenticated',
					oauthAccessToken: accessToken,
					oauthRefreshToken: refreshToken || null,
					oauthExpiresAt: expiresAt,
					scopes: requiredScopes,
					sourceMetadata: {
						connectedAt: new Date().toISOString(),
						provider: provider
					}
				})
				.returning();
			sourceInstance = newSource;
			console.log(`OAuth connection successful: created source instance ${newSource.id} for ${sourceName}`);
		}

		// In the stream-based architecture, we don't auto-activate anything
		// The user will see available streams and activate them manually

		// Redirect back to the original page with success message
		const successUrl = returnPath.includes('?') 
			? `${returnPath}&connected=${sourceName}`
			: `${returnPath}?connected=${sourceName}`;
		return redirect(302, successUrl);

	} catch (err) {
		// If it's a redirect, just throw it (don't log as error)
		if (err && typeof err === 'object' && 'status' in err && 'location' in err) {
			throw err;
		}

		console.error('Error processing OAuth callback:', err);
		return redirect(302, returnPath + '?oauth_error=' + encodeURIComponent('Connection failed. Please try again.'));
	}
};