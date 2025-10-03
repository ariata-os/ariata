import { 
  parseAbsolute,
  parseDate,
  toZoned,
  toCalendarDateTime,
  now,
  type ZonedDateTime 
} from '@internationalized/date';

/**
 * Convert a UTC Date to a ZonedDateTime in the user's timezone
 */
export function toUserTimezone(utcDate: Date | string, timezone: string): ZonedDateTime {
  // Convert to ISO string if needed
  const isoString = typeof utcDate === 'string' 
    ? utcDate 
    : utcDate.toISOString();
  
  // Parse as UTC and convert to user timezone
  const utcDateTime = parseAbsolute(isoString, 'UTC');
  return toZoned(utcDateTime, timezone);
}

/**
 * Get hours (0-23) in user's timezone
 */
export function getHoursInTimezone(utcDate: Date | string, timezone: string): number {
  const zonedDate = toUserTimezone(utcDate, timezone);
  return zonedDate.hour;
}

/**
 * Get minutes (0-59) in user's timezone
 */
export function getMinutesInTimezone(utcDate: Date | string, timezone: string): number {
  const zonedDate = toUserTimezone(utcDate, timezone);
  return zonedDate.minute;
}

/**
 * Format time in user's timezone
 */
export function formatTimeInTimezone(utcDate: Date | string, timezone: string, options?: Intl.DateTimeFormatOptions): string {
  const zonedDate = toUserTimezone(utcDate, timezone);
  
  // Default formatting options
  const defaultOptions: Intl.DateTimeFormatOptions = {
    hour: 'numeric',
    minute: '2-digit',
    hour12: true,
    timeZone: timezone
  };
  
  // Create a JavaScript Date from the ZonedDateTime for formatting
  const jsDate = zonedDate.toDate();
  
  return jsDate.toLocaleTimeString('en-US', options || defaultOptions);
}

/**
 * Convert a pixel position back to a UTC Date given the base date and timezone
 */
export function pixelToUTCDate(
  totalMinutes: number, 
  baseDate: Date, 
  timezone: string
): Date {
  // Parse base date in the user's timezone at midnight
  const year = baseDate.getFullYear();
  const month = baseDate.getMonth() + 1;
  const day = baseDate.getDate();
  
  // Create a CalendarDate object
  const calendarDate = parseDate(`${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`);
  
  // Convert to ZonedDateTime at midnight in the user's timezone
  const zonedBase = toZoned(calendarDate, timezone);
  
  // Add the minutes
  const targetDateTime = zonedBase.add({ minutes: totalMinutes });
  
  // Convert back to UTC Date
  return targetDateTime.toDate();
}

/**
 * Create a Date for a specific hour on a given day in the user's timezone
 */
export function createDateForHour(
  year: number,
  month: number,
  day: number,
  hour: number,
  timezone: string
): Date {
  // Create a CalendarDate object
  const calendarDate = parseDate(`${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`);
  
  // Convert to ZonedDateTime at the specified hour in the user's timezone
  const zonedDateTime = toZoned(calendarDate, timezone).set({ hour, minute: 0, second: 0 });
  
  // Convert to UTC Date
  return zonedDateTime.toDate();
}