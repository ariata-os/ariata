import express, { Request, Response } from 'express';
import { oauthConfigs } from '../config/oauth-apps';
import { createError } from '../middleware/error-handler';

const router = express.Router();

// Generate state parameter for CSRF protection
const generateState = () => {
  return Math.random().toString(36).substring(2, 15) + 
         Math.random().toString(36).substring(2, 15);
};

// Initiate Strava OAuth flow
router.get('/auth', (req: Request, res: Response) => {
  try {
    const { return_url, state: originalState } = req.query;
    
    if (!return_url || typeof return_url !== 'string') {
      throw createError('Missing return_url parameter', 400);
    }
    
    // Validate return_url to prevent open redirect attacks
    if (!isValidReturnUrl(return_url)) {
      throw createError('Invalid return_url parameter', 400);
    }
    
    const state = generateState();
    const config = oauthConfigs.strava;
    
    // Debug: Check if client_id is loaded
    console.log('Strava OAuth config:', {
      clientId: config.clientId ? 'SET' : 'MISSING',
      clientSecret: config.clientSecret ? 'SET' : 'MISSING',
      redirectUri: config.redirectUri
    });
    
    // Store state and return_url (in production, use Redis or similar)
    // For now, encode in state parameter
    const stateData = {
      state: originalState || state,  // Use original state if provided
      return_url,
      timestamp: Date.now()
    };
    
    const encodedState = Buffer.from(JSON.stringify(stateData)).toString('base64');
    
    const authUrl = new URL(config.authUrl);
    authUrl.searchParams.set('client_id', config.clientId);
    authUrl.searchParams.set('redirect_uri', config.redirectUri);
    authUrl.searchParams.set('scope', config.scopes.join(' ')); // Strava uses comma-separated scopes
    authUrl.searchParams.set('response_type', 'code');
    authUrl.searchParams.set('approval_prompt', 'auto'); // Strava-specific parameter
    authUrl.searchParams.set('state', encodedState);
    
    res.redirect(authUrl.toString());
    
  } catch (error) {
    console.error('Strava auth error:', error);
    res.status(500).json({ error: 'Failed to initiate Strava OAuth' });
  }
});

// Handle Strava OAuth callback
router.get('/callback', async (req: Request, res: Response) => {
  try {
    const { code, state, error } = req.query;
    
    if (error) {
      throw createError(`OAuth error: ${error}`, 400);
    }
    
    if (!code || !state) {
      throw createError('Missing code or state parameter', 400);
    }
    
    // Decode state to get return_url and original state
    const stateData = JSON.parse(Buffer.from(state as string, 'base64').toString());
    const { return_url, state: originalState } = stateData;
    
    if (!return_url) {
      throw createError('Invalid state parameter', 400);
    }
    
    // Validate return_url again
    if (!isValidReturnUrl(return_url)) {
      throw createError('Invalid return_url in state', 400);
    }
    
    // Exchange code for tokens HERE in the auth-proxy
    const tokens = await exchangeCodeForTokens(code as string);
    
    // Redirect back to user's instance with the tokens
    const returnUrl = new URL(return_url);
    returnUrl.searchParams.set('access_token', tokens.access_token);
    if (tokens.refresh_token) {
      returnUrl.searchParams.set('refresh_token', tokens.refresh_token);
    }
    if (tokens.expires_at) {
      returnUrl.searchParams.set('expires_at', tokens.expires_at.toString());
    }
    if (tokens.expires_in) {
      returnUrl.searchParams.set('expires_in', tokens.expires_in.toString());
    }
    returnUrl.searchParams.set('provider', 'strava');
    // Pass the original state back to the user's callback
    if (originalState) {
      returnUrl.searchParams.set('state', originalState);
    }
    
    res.redirect(returnUrl.toString());
    
  } catch (error) {
    console.error('Strava callback error:', error);
    
    // Redirect to user's instance with error
    try {
      const stateData = JSON.parse(Buffer.from(req.query.state as string, 'base64').toString());
      const returnUrl = new URL(stateData.return_url);
      returnUrl.searchParams.set('error', 'token_exchange_failed');
      res.redirect(returnUrl.toString());
    } catch {
      res.status(500).json({ error: 'Failed to process Strava OAuth callback' });
    }
  }
});

// Exchange authorization code for tokens
async function exchangeCodeForTokens(code: string) {
  const config = oauthConfigs.strava;
  
  const body = new URLSearchParams({
    code,
    client_id: config.clientId,
    client_secret: config.clientSecret,
    grant_type: 'authorization_code'
  });

  const response = await fetch(config.tokenUrl, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/x-www-form-urlencoded'
    },
    body: body.toString()
  });

  if (!response.ok) {
    const errorData = await response.text();
    throw new Error(`Token exchange failed: ${response.status} ${errorData}`);
  }

  const tokens = await response.json();
  
  if (!tokens.access_token) {
    throw new Error('No access token received');
  }

  return tokens;
}

// Refresh access token using refresh token
router.post('/refresh', async (req: Request, res: Response) => {
  try {
    const { refresh_token } = req.body;
    
    if (!refresh_token) {
      throw createError('Missing required parameter: refresh_token', 400);
    }
    
    // Use the auth proxy's own OAuth credentials
    const config = oauthConfigs.strava;
    
    const body = new URLSearchParams({
      refresh_token,
      client_id: config.clientId,
      client_secret: config.clientSecret,
      grant_type: 'refresh_token'
    });

    const response = await fetch(config.tokenUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded'
      },
      body: body.toString()
    });

    if (!response.ok) {
      const errorData = await response.text();
      console.error('Token refresh failed:', response.status, errorData);
      
      // Check if it's an invalid_grant error (refresh token expired or revoked)
      if (errorData.includes('invalid_grant')) {
        throw createError('Refresh token is invalid or expired', 401);
      }
      
      throw createError(`Token refresh failed: ${response.status}`, response.status);
    }

    const tokens: any = await response.json();
    
    if (!tokens.access_token) {
      throw createError('No access token received from refresh', 500);
    }

    // Return the new tokens (Strava includes expires_at as Unix timestamp)
    res.json({
      access_token: tokens.access_token,
      refresh_token: tokens.refresh_token || refresh_token, // Strava always returns a new refresh token
      expires_at: tokens.expires_at,
      expires_in: tokens.expires_in || 21600, // 6 hours default for Strava
      token_type: tokens.token_type || 'Bearer'
    });
    
  } catch (error: any) {
    console.error('Token refresh error:', error);
    
    if (error.statusCode) {
      res.status(error.statusCode).json({ 
        error: error.message,
        code: error.statusCode === 401 ? 'invalid_refresh_token' : 'refresh_failed'
      });
    } else {
      res.status(500).json({ 
        error: 'Failed to refresh token',
        code: 'refresh_failed'
      });
    }
  }
});

// Validate return URL to prevent open redirect attacks
function isValidReturnUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    
    // Allow localhost for development
    if (parsed.hostname === 'localhost' || parsed.hostname === '127.0.0.1') {
      return true;
    }
    
    // Allow specific domains (add your domain patterns here)
    const allowedPatterns = [
      /^.*\.jaces\.com$/,
      /^.*\.local$/,
      /^.*\.localhost$/
    ];
    
    return allowedPatterns.some(pattern => pattern.test(parsed.hostname));
  } catch {
    return false;
  }
}

export { router as stravaRouter };