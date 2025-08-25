/**
 * Converts a cron expression to minutes for frequency comparison
 * Returns the interval in minutes between executions
 * Uses simple pattern matching for common cron expressions
 */
export function cronToMinutes(cronExpression: string): number {
  const expr = cronExpression.trim();
  
  // Handle common patterns
  if (expr.match(/^\*\/(\d+) \* \* \* \*$/)) {
    // Every N minutes: */5 * * * *
    const minutes = parseInt(expr.match(/^\*\/(\d+) \* \* \* \*$/)![1]);
    return minutes;
  }
  
  if (expr.match(/^0 \*\/(\d+) \* \* \*$/)) {
    // Every N hours: 0 */2 * * *
    const hours = parseInt(expr.match(/^0 \*\/(\d+) \* \* \*$/)![1]);
    return hours * 60;
  }
  
  if (expr === '0 * * * *') {
    return 60; // Every hour
  }
  
  if (expr === '0 0 * * *') {
    return 1440; // Every day
  }
  
  if (expr === '0 0 * * 0') {
    return 10080; // Every week
  }
  
  // Default fallback - estimate based on first field
  const fields = expr.split(' ');
  if (fields.length >= 1) {
    const minuteField = fields[0];
    if (minuteField.startsWith('*/')) {
      return parseInt(minuteField.substring(2));
    }
    if (minuteField === '0') {
      return 60; // Assume hourly if starts with 0
    }
  }
  
  throw new Error(`Unsupported cron expression: ${cronExpression}`);
}

/**
 * Checks if a source has locked scheduling (min === max frequency)
 */
export function isSourceScheduleLocked(minFreq: string | null, maxFreq: string | null): boolean {
  if (!minFreq || !maxFreq) return false;
  return minFreq === maxFreq;
}

/**
 * Validates if a cron schedule is within the allowed frequency range for a source
 */
export function isScheduleWithinBounds(
  schedule: string,
  minFreq: string | null,
  maxFreq: string | null
): { valid: boolean; error?: string } {
  try {
    if (!minFreq || !maxFreq) {
      // No constraints defined, allow any valid cron
      cronToMinutes(schedule); // Validate syntax
      return { valid: true };
    }

    const scheduleMinutes = cronToMinutes(schedule);
    const minMinutes = cronToMinutes(minFreq);
    const maxMinutes = cronToMinutes(maxFreq);

    if (scheduleMinutes < minMinutes) {
      return {
        valid: false,
        error: `Schedule too frequent. Minimum interval is ${minMinutes} minutes.`
      };
    }

    if (scheduleMinutes > maxMinutes) {
      return {
        valid: false,
        error: `Schedule too infrequent. Maximum interval is ${maxMinutes} minutes.`
      };
    }

    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof Error ? error.message : 'Invalid cron expression'
    };
  }
}

/**
 * Gets the human-readable frequency description for locked sources
 */
export function getLockedFrequencyDescription(cronExpression: string): string {
  try {
    const minutes = cronToMinutes(cronExpression);
    
    if (minutes < 60) {
      return `${minutes} minute${minutes !== 1 ? 's' : ''}`;
    } else if (minutes < 1440) {
      const hours = minutes / 60;
      return `${hours} hour${hours !== 1 ? 's' : ''}`;
    } else {
      const days = minutes / 1440;
      return `${days} day${days !== 1 ? 's' : ''}`;
    }
  } catch (error) {
    return 'fixed interval';
  }
}

/**
 * Filters cron schedule options based on source constraints
 */
export function getValidScheduleOptions(
  minFreq: string | null,
  maxFreq: string | null
): Array<{ label: string; value: string; minutes: number }> {
  const allOptions = [
    { label: '5 minutes', value: '*/5 * * * *', minutes: 5 },
    { label: '10 minutes', value: '*/10 * * * *', minutes: 10 },
    { label: '15 minutes', value: '*/15 * * * *', minutes: 15 },
    { label: '30 minutes', value: '*/30 * * * *', minutes: 30 },
    { label: '45 minutes', value: '*/45 * * * *', minutes: 45 },
    { label: '1 hour', value: '0 * * * *', minutes: 60 },
    { label: '2 hours', value: '0 */2 * * *', minutes: 120 },
    { label: '4 hours', value: '0 */4 * * *', minutes: 240 },
    { label: '8 hours', value: '0 */8 * * *', minutes: 480 },
    { label: '12 hours', value: '0 */12 * * *', minutes: 720 },
    { label: '1 day', value: '0 0 * * *', minutes: 1440 },
    { label: '1 week', value: '0 0 * * 0', minutes: 10080 }
  ];

  if (!minFreq || !maxFreq) {
    return allOptions;
  }

  try {
    const minMinutes = cronToMinutes(minFreq);
    const maxMinutes = cronToMinutes(maxFreq);

    return allOptions.filter(option => 
      option.minutes >= minMinutes && option.minutes <= maxMinutes
    );
  } catch (error) {
    // If we can't parse constraints, return all options
    return allOptions;
  }
}

/**
 * Calculates the next execution time for a cron expression from a given starting time
 */
export function getNextCronExecution(cronExpression: string, fromTime: Date = new Date()): Date {
  try {
    const intervalMinutes = cronToMinutes(cronExpression);
    const nextExecution = new Date(fromTime.getTime() + intervalMinutes * 60 * 1000);
    return nextExecution;
  } catch (error) {
    // Fallback to 1 hour from now if we can't parse
    return new Date(fromTime.getTime() + 60 * 60 * 1000);
  }
}

/**
 * Formats time difference as relative time (e.g., "2 minutes ago", "in 5 minutes")
 */
export function formatRelativeTime(date: Date, now: Date = new Date()): string {
  const diffMs = date.getTime() - now.getTime();
  const diffMinutes = Math.round(diffMs / (1000 * 60));
  const diffHours = Math.round(diffMs / (1000 * 60 * 60));
  const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));

  if (Math.abs(diffMs) < 60 * 1000) {
    return diffMs < 0 ? 'just now' : 'now';
  }

  if (Math.abs(diffMinutes) < 60) {
    if (diffMinutes < 0) {
      const abs = Math.abs(diffMinutes);
      return `${abs} minute${abs !== 1 ? 's' : ''} ago`;
    } else {
      return `in ${diffMinutes} minute${diffMinutes !== 1 ? 's' : ''}`;
    }
  }

  if (Math.abs(diffHours) < 24) {
    if (diffHours < 0) {
      const abs = Math.abs(diffHours);
      return `${abs} hour${abs !== 1 ? 's' : ''} ago`;
    } else {
      return `in ${diffHours} hour${diffHours !== 1 ? 's' : ''}`;
    }
  }

  if (diffDays < 0) {
    const abs = Math.abs(diffDays);
    return `${abs} day${abs !== 1 ? 's' : ''} ago`;
  } else {
    return `in ${diffDays} day${diffDays !== 1 ? 's' : ''}`;
  }
}

/**
 * Formats countdown timer (e.g., "5m 32s", "1h 5m", "2d 3h")
 */
export function formatCountdown(date: Date, now: Date = new Date()): string {
  const diffMs = date.getTime() - now.getTime();
  
  if (diffMs <= 0) {
    return 'now';
  }

  const totalSeconds = Math.floor(diffMs / 1000);
  const days = Math.floor(totalSeconds / (24 * 60 * 60));
  const hours = Math.floor((totalSeconds % (24 * 60 * 60)) / (60 * 60));
  const minutes = Math.floor((totalSeconds % (60 * 60)) / 60);
  const seconds = totalSeconds % 60;

  if (days > 0) {
    return `${days}d ${hours}h`;
  } else if (hours > 0) {
    return `${hours}h ${minutes}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${seconds}s`;
  } else {
    return `${seconds}s`;
  }
}