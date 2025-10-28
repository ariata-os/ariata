import { Request as ExpressRequest, Response as ExpressResponse, NextFunction } from 'express';

export const logger = (req: ExpressRequest, res: ExpressResponse, next: NextFunction) => {
  const start = Date.now();
  
  res.on('finish', () => {
    const duration = Date.now() - start;
    const timestamp = new Date().toISOString();
    
    console.log(
      `[${timestamp}] ${req.method} ${req.path} ${res.statusCode} - ${duration}ms`
    );
  });
  
  next();
};