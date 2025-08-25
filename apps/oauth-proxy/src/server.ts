import dotenv from 'dotenv';
dotenv.config(); // Load environment variables FIRST

import express from 'express';
import cors from 'cors';
import helmet from 'helmet';
import rateLimit from 'express-rate-limit';

import { googleRouter } from './routes/google';
import notionRouter from './routes/notion';
import { stravaRouter } from './routes/strava';
import { errorHandler } from './middleware/error-handler';
import { logger } from './middleware/logger';

const app = express();
const PORT = process.env.PORT || 3000;

// Security middleware
app.use(helmet());
app.use(cors({
  origin: process.env.ALLOWED_ORIGINS?.split(',') || ['http://localhost:5173'],
  credentials: true
}));

// Rate limiting
const limiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100, // limit each IP to 100 requests per windowMs
  message: 'Too many requests from this IP, please try again later.'
});
app.use(limiter);

// Middleware
app.use(express.json());
app.use(logger);

// Health check
app.get('/health', (req, res) => {
  res.json({ status: 'ok', timestamp: new Date().toISOString() });
});

// OAuth routes
app.use('/google', googleRouter);
app.use('/notion', notionRouter);
app.use('/strava', stravaRouter);

// Error handling
app.use(errorHandler);

// 404 handler
app.use('*', (req, res) => {
  res.status(404).json({ error: 'Route not found' });
});

app.listen(PORT, () => {
  console.log(`🚀 OAuth proxy server running on port ${PORT}`);
  console.log(`🌐 Environment: ${process.env.NODE_ENV || 'development'}`);
  console.log(`📦 Providers: Google, Notion, Microsoft, GitHub, Strava`);
});

export default app;