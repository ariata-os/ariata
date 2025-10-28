import { Request, Response, NextFunction } from 'express';
export interface AppError extends Error {
    statusCode?: number;
    isOperational?: boolean;
}
export declare const errorHandler: (err: AppError, req: Request, res: Response, next: NextFunction) => void;
export declare const createError: (message: string, statusCode: number) => AppError;
//# sourceMappingURL=error-handler.d.ts.map