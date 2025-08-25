import { Router, Request, Response } from 'express';
import crypto from 'crypto';
import { oauthConfigs } from '../config/oauth-apps';

// Type for Notion OAuth token response
interface NotionTokenResponse {
  access_token: string;
  workspace_id?: string;
  workspace_name?: string;
  bot_id?: string;
}

const router: Router = Router();

// In-memory store for state parameters (in production, use Redis or similar)
const stateStore = new Map<string, { returnUrl: string; timestamp: number }>();

// Clean up old state entries every 5 minutes
setInterval(() => {
  const now = Date.now();
  for (const [state, data] of stateStore.entries()) {
    if (now - data.timestamp > 10 * 60 * 1000) { // 10 minutes
      stateStore.delete(state);
    }
  }
}, 5 * 60 * 1000);

/**
 * Initiate Notion OAuth flow
 * @route GET /notion/auth
 * @param return_url - The URL to redirect back to after auth
 */
router.get('/auth', (req: Request, res: Response) => {
  const returnUrl = req.query.return_url as string;
  const originalState = req.query.state as string;

  if (!returnUrl) {
    return res.status(400).json({ error: 'Missing return_url parameter' });
  }

  // Generate state for CSRF protection
  const state = crypto.randomBytes(32).toString('hex');
  stateStore.set(state, { returnUrl, originalState, timestamp: Date.now() });

  const config = oauthConfigs.notion;

  // Build Notion OAuth URL
  const params = new URLSearchParams({
    client_id: config.clientId,
    redirect_uri: config.redirectUri,
    response_type: 'code',
    state: state,
    owner: 'user' // Notion-specific: 'user' for personal integrations
  });

  const authUrl = `${config.authUrl}?${params.toString()}`;

  console.log('Redirecting to Notion OAuth:', authUrl);
  res.redirect(authUrl);
});

/**
 * Handle Notion OAuth callback
 * @route GET /notion/callback
 */
router.get('/callback', async (req: Request, res: Response) => {
  const { code, state, error } = req.query;

  if (error) {
    console.error('Notion OAuth error:', error);
    return res.status(400).send(`OAuth error: ${error}`);
  }

  if (!code || !state) {
    return res.status(400).json({ error: 'Missing code or state parameter' });
  }

  // Verify state
  const stateData = stateStore.get(state as string);
  if (!stateData) {
    return res.status(400).json({ error: 'Invalid state parameter' });
  }

  // Clean up state
  stateStore.delete(state as string);

  const config = oauthConfigs.notion;

  try {
    // Exchange code for access token
    const tokenResponse = await fetch(config.tokenUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Basic ${Buffer.from(`${config.clientId}:${config.clientSecret}`).toString('base64')}`
      },
      body: JSON.stringify({
        grant_type: 'authorization_code',
        code: code as string,
        redirect_uri: config.redirectUri
      })
    });

    if (!tokenResponse.ok) {
      const error = await tokenResponse.text();
      console.error('Token exchange failed:', error);
      throw new Error(`Token exchange failed: ${tokenResponse.status}`);
    }

    const tokenData = await tokenResponse.json() as NotionTokenResponse;

    // Redirect back to the user's instance with the access token
    const redirectUrl = new URL(stateData.returnUrl);
    redirectUrl.searchParams.set('access_token', tokenData.access_token);
    redirectUrl.searchParams.set('workspace_id', tokenData.workspace_id || '');
    redirectUrl.searchParams.set('workspace_name', tokenData.workspace_name || '');
    redirectUrl.searchParams.set('bot_id', tokenData.bot_id || '');
    redirectUrl.searchParams.set('provider', 'notion');
    // Pass the original state back to the user's callback
    if (stateData.originalState) {
      redirectUrl.searchParams.set('state', stateData.originalState);
    }

    console.log('Redirecting back to:', redirectUrl.toString());
    res.redirect(redirectUrl.toString());
  } catch (error) {
    console.error('Error exchanging code for token:', error);
    res.status(500).json({ error: 'Failed to exchange code for token' });
  }
});

/**
 * Refresh Notion access token
 * @route POST /notion/refresh
 */
router.post('/refresh', async (req: Request, res: Response) => {
  const { refresh_token } = req.body;

  if (!refresh_token) {
    return res.status(400).json({ error: 'Missing refresh_token' });
  }

  const config = oauthConfigs.notion;

  try {
    // Note: Notion doesn't currently support refresh tokens
    // Access tokens don't expire, so this endpoint is a placeholder
    res.json({
      message: 'Notion access tokens do not expire and cannot be refreshed',
      access_token: refresh_token // Return the same token
    });
  } catch (error) {
    console.error('Error refreshing token:', error);
    res.status(500).json({ error: 'Failed to refresh token' });
  }
});

export default router;