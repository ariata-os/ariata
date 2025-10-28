"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const express_1 = __importDefault(require("express"));
const oauth_apps_1 = require("../config/oauth-apps");
const error_handler_1 = require("../middleware/error-handler");
const router = express_1.default.Router();
// In-memory store for state parameters (in production, use Redis or similar)
const stateStore = new Map();
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
router.get('/auth', (req, res) => {
    const returnUrl = req.query.return_url;
    const originalState = req.query.state;
    if (!returnUrl) {
        return res.status(400).json({ error: 'Missing return_url parameter' });
    }
    // Encode state data in the state parameter (like Google/Strava) to work with serverless
    const stateData = {
        return_url: returnUrl,
        state: originalState,
        timestamp: Date.now()
    };
    const encodedState = Buffer.from(JSON.stringify(stateData)).toString('base64');
    const config = oauth_apps_1.oauthConfigs.notion;
    // Build Notion OAuth URL
    const params = new URLSearchParams({
        client_id: config.clientId,
        redirect_uri: config.redirectUri,
        response_type: 'code',
        state: encodedState,
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
router.get('/callback', async (req, res) => {
    try {
        const { code, state, error } = req.query;
        if (error) {
            throw (0, error_handler_1.createError)(`OAuth error: ${error}`, 400);
        }
        if (!code || !state) {
            throw (0, error_handler_1.createError)('Missing code or state parameter', 400);
        }
        // Decode state from parameter (serverless-compatible)
        const stateData = JSON.parse(Buffer.from(state, 'base64').toString());
        const { return_url, state: originalState, timestamp } = stateData;
        if (!return_url) {
            throw (0, error_handler_1.createError)('Invalid state parameter', 400);
        }
        // Validate timestamp (reject if > 10 minutes old)
        if (Date.now() - timestamp > 10 * 60 * 1000) {
            throw (0, error_handler_1.createError)('State parameter expired', 400);
        }
        // Validate return URL
        if (!isValidReturnUrl(return_url)) {
            throw (0, error_handler_1.createError)('Invalid return URL', 400);
        }
        const config = oauth_apps_1.oauthConfigs.notion;
        // Debug: Log configuration (without exposing secrets)
        console.log('Notion token exchange config:', {
            tokenUrl: config.tokenUrl,
            redirectUri: config.redirectUri,
            clientIdSet: !!config.clientId,
            clientSecretSet: !!config.clientSecret,
            clientIdLength: config.clientId?.length,
            code: code
        });
        // Exchange code for access token
        // Note: Notion requires form-encoded body, not JSON
        const tokenResponse = await fetch(config.tokenUrl, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
                'Authorization': `Basic ${Buffer.from(`${config.clientId}:${config.clientSecret}`).toString('base64')}`
            },
            body: new URLSearchParams({
                grant_type: 'authorization_code',
                code: code,
                redirect_uri: config.redirectUri
            }).toString()
        });
        if (!tokenResponse.ok) {
            const error = await tokenResponse.text();
            console.error('Notion token exchange failed:', {
                status: tokenResponse.status,
                statusText: tokenResponse.statusText,
                error: error
            });
            throw new Error(`Token exchange failed: ${tokenResponse.status} - ${error}`);
        }
        const tokenData = await tokenResponse.json();
        // Redirect back to the user's instance with the access token
        const redirectUrl = new URL(return_url);
        redirectUrl.searchParams.set('access_token', tokenData.access_token);
        redirectUrl.searchParams.set('workspace_id', tokenData.workspace_id || '');
        redirectUrl.searchParams.set('workspace_name', tokenData.workspace_name || '');
        redirectUrl.searchParams.set('bot_id', tokenData.bot_id || '');
        redirectUrl.searchParams.set('provider', 'notion');
        // Pass the original state back to the user's callback
        if (originalState) {
            redirectUrl.searchParams.set('state', originalState);
        }
        console.log('Redirecting back to:', redirectUrl.toString());
        res.redirect(redirectUrl.toString());
    }
    catch (error) {
        console.error('Error exchanging code for token:', error);
        res.status(500).json({ error: 'Failed to exchange code for token' });
    }
});
/**
 * Refresh Notion access token
 * @route POST /notion/refresh
 */
router.post('/refresh', async (req, res) => {
    const { refresh_token } = req.body;
    if (!refresh_token) {
        return res.status(400).json({ error: 'Missing refresh_token' });
    }
    const config = oauth_apps_1.oauthConfigs.notion;
    try {
        // Note: Notion doesn't currently support refresh tokens
        // Access tokens don't expire, so this endpoint is a placeholder
        res.json({
            message: 'Notion access tokens do not expire and cannot be refreshed',
            access_token: refresh_token // Return the same token
        });
    }
    catch (error) {
        console.error('Error refreshing token:', error);
        res.status(500).json({ error: 'Failed to refresh token' });
    }
});
/**
 * Exchange authorization code for access token
 * Used by CLI and other clients that can't use the redirect flow
 * @route POST /notion/token
 */
router.post('/token', async (req, res) => {
    const { code } = req.body;
    if (!code) {
        return res.status(400).json({ error: 'Missing code parameter' });
    }
    const config = oauth_apps_1.oauthConfigs.notion;
    try {
        // Debug: Log configuration (without exposing secrets)
        console.log('Notion /token endpoint config:', {
            tokenUrl: config.tokenUrl,
            redirectUri: config.redirectUri,
            clientIdSet: !!config.clientId,
            clientSecretSet: !!config.clientSecret,
            clientIdLength: config.clientId?.length,
            code
        });
        // Exchange code for access token
        // Note: Notion requires form-encoded body, not JSON
        const tokenResponse = await fetch(config.tokenUrl, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
                'Authorization': `Basic ${Buffer.from(`${config.clientId}:${config.clientSecret}`).toString('base64')}`
            },
            body: new URLSearchParams({
                grant_type: 'authorization_code',
                code: code,
                redirect_uri: config.redirectUri
            }).toString()
        });
        if (!tokenResponse.ok) {
            const error = await tokenResponse.text();
            console.error('Notion /token exchange failed:', {
                status: tokenResponse.status,
                statusText: tokenResponse.statusText,
                error: error
            });
            return res.status(tokenResponse.status).json({ error: 'Token exchange failed', details: error });
        }
        const tokenData = await tokenResponse.json();
        res.json(tokenData);
    }
    catch (error) {
        console.error('Error exchanging code for token:', error);
        res.status(500).json({ error: 'Failed to exchange code for token' });
    }
});
// Helper function to validate return URLs
function isValidReturnUrl(url) {
    try {
        const parsed = new URL(url);
        // Allow localhost for development
        if (parsed.hostname === 'localhost' || parsed.hostname === '127.0.0.1') {
            return true;
        }
        // Allow specific domains
        const allowedPatterns = [
            /^.*\.ariata\.com$/,
            /^.*\.local$/,
            /^.*\.localhost$/
        ];
        return allowedPatterns.some(pattern => pattern.test(parsed.hostname));
    }
    catch {
        return false;
    }
}
// Temporary debug endpoint - REMOVE IN PRODUCTION
router.get('/debug-config', (req, res) => {
    const config = oauth_apps_1.oauthConfigs.notion;
    res.json({
        tokenUrl: config.tokenUrl,
        redirectUri: config.redirectUri,
        clientIdSet: !!config.clientId,
        clientSecretSet: !!config.clientSecret,
        clientIdLength: config.clientId?.length || 0,
        clientIdPrefix: config.clientId?.substring(0, 8) + '...',
        clientSecretLength: config.clientSecret?.length || 0
    });
});
exports.default = router;
//# sourceMappingURL=notion.js.map