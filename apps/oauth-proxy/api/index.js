// Vercel serverless function entry point
// This wraps the Express app to work with Vercel's serverless functions
const app = require('../dist/server').default;

module.exports = app;
