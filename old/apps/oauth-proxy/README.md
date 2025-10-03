# Ariata OAuth Proxy

Unified OAuth proxy service for multiple providers (Google, Notion, Microsoft, etc.). This service handles OAuth flows for various providers without storing user data.

## Overview

The auth proxy acts as a bridge between self-hosted Ariata instances and OAuth providers like Google, Microsoft, and GitHub. It handles the complex OAuth flows while keeping user data sovereignty intact.

## Architecture

```
Self-hosted Ariata → auth.ariata.com → OAuth Provider → auth.ariata.com → Self-hosted Ariata
```

**Key principles:**
- **No data storage**: Only facilitates OAuth handshakes
- **Stateless**: No user sessions or persistent data
- **Secure**: CSRF protection, rate limiting, and URL validation
- **Transparent**: Open source for full auditability

## Supported Providers

- ✅ Google (Calendar, Gmail, Drive)
- 🚧 Notion (Pages, Databases)
- 🚧 Microsoft (Outlook, OneDrive, Teams)
- 🚧 GitHub (Repositories, Issues)

## API Endpoints

### Google OAuth
- `GET /google/auth?return_url=<user_instance_url>` - Initiate Google OAuth flow
- `GET /google/callback` - Handle Google OAuth callback

### Health Check
- `GET /health` - Service health status

## Environment Variables

```bash
# Required
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret

# Optional
PORT=3000
NODE_ENV=development
ALLOWED_ORIGINS=http://localhost:5173,http://localhost:3000
GOOGLE_REDIRECT_URI=https://auth.ariata.com/google/callback
```

## Development

```bash
# Install dependencies
npm install

# Copy environment variables
cp .env.example .env

# Start development server
npm run dev
```

## Deployment

### Vercel
```bash
# Deploy to Vercel
vercel --prod
```

### Docker
```bash
# Build and run with Docker
docker build -t ariata-oauth-proxy .
docker run -p 3000:3000 --env-file .env ariata-oauth-proxy
```

## Security Features

- **CSRF Protection**: State parameter validation
- **Rate Limiting**: 100 requests per 15 minutes per IP
- **URL Validation**: Prevents open redirect attacks
- **CORS**: Configurable allowed origins
- **Helmet**: Security headers

## Usage from Self-hosted Ariata

```typescript
// Redirect user to OAuth proxy
const authUrl = `https://auth.ariata.com/google/auth?return_url=${encodeURIComponent(callbackUrl)}`;
window.location.href = authUrl;

// Handle callback in your instance
app.get('/oauth/callback', (req, res) => {
  const { code, provider } = req.query;
  // Exchange code for tokens directly with the provider
});
```