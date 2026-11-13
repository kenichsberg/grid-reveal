/** Shared formatting helpers. */

/** `135` -> `"2h 15m"`, `45` -> `"45m"`, `120` -> `"2h"`. */
export function formatMinutes(total: number): string {
  if (!Number.isFinite(total) || total <= 0) return '0m';
  const hours = Math.floor(total / 60);
  const minutes = total % 60;
  if (hours === 0) return `${minutes}m`;
  if (minutes === 0) return `${hours}h`;
  return `${hours}h ${minutes}m`;
}

/**
 * `750` -> `"12.5"`, `720` -> `"12"`, `0` -> `"0"`.
 *
 * For the board's current/goal readout, where the unit is written separately
 * and a running "12h 30m" would be two numbers competing for one glance.
 */
export function formatHours(total: number): string {
  if (!Number.isFinite(total) || total <= 0) return '0';
  const hours = total / 60;
  // One decimal, but never a bare `.0`: whole hours should read as whole.
  return (Math.round(hours * 10) / 10).toFixed(1).replace(/\.0$/, '');
}

/** Splits stored minutes back into the hours/minutes input pair. */
export function splitMinutes(total: number): { hours: number; minutes: number } {
  return { hours: Math.floor(total / 60), minutes: total % 60 };
}

/** Today as `YYYY-MM-DD` in the viewer's own calendar. */
export function today(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** `2026-11-13` -> `13 Nov 2026`, parsed as a plain date with no timezone shift. */
export function formatDate(iso: string): string {
  const [y, m, d] = iso.split('-').map(Number);
  if (!y || !m || !d) return iso;
  return new Date(y, m - 1, d).toLocaleDateString(undefined, {
    day: 'numeric',
    month: 'short',
    year: 'numeric'
  });
}

export function percent(part: number, whole: number): number {
  if (whole <= 0) return 0;
  return Math.round((part / whole) * 100);
}

/**
 * `2026-12-31` -> `31 Dec`, with the year appended when it is not this one.
 *
 * For dates shown inside running prose ("Ends 31 Dec"), where the full
 * `formatDate` form is more precision than the sentence can carry.
 */
export function formatDayMonth(iso: string): string {
  const [y, m, d] = iso.split('-').map(Number);
  if (!y || !m || !d) return iso;
  return new Date(y, m - 1, d).toLocaleDateString(undefined, {
    day: 'numeric',
    month: 'short',
    ...(y === new Date().getFullYear() ? {} : { year: 'numeric' })
  });
}
