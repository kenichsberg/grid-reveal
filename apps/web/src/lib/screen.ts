/**
 * What a screen's lifecycle phase permits, in one place.
 *
 * The server derives `phase` from the stored window and its own clock. The
 * client renders that verbatim and never recomputes it from `starts_on` /
 * `ends_on`: a second derivation could only ever disagree with the first —
 * across a timezone's day boundary, say — and the member would be left with a
 * button that lies about whether it works.
 */
import type { Phase, ScreenDetail } from '$lib/api/types';
import { formatDayMonth } from '$lib/format';

/** The fields these helpers read; keeps them usable from the list page too. */
type Lifecycle = {
  phase: Phase;
  starts_on: string | null;
  ends_on: string | null;
};

/**
 * Whether progress may be logged right now.
 *
 * Mirrors the API's single write gate: only an active screen accepts writes.
 * An upcoming, ended or archived screen is fully read-only.
 */
export function canLog(screen: Lifecycle): boolean {
  return screen.phase === 'active';
}

/**
 * Whether goals may be added or removed right now.
 *
 * Identical to {@link canLog} by design, and named separately only because the
 * two are different questions to the reader. When a period closes the whole
 * screen freezes, goals included.
 */
export function canEditGoals(screen: Lifecycle): boolean {
  return screen.phase === 'active';
}

/** Total hours needed to reveal every tile. */
export function goalMinutes(screen: Pick<ScreenDetail, 'total_tiles' | 'unit_minutes'>): number {
  return screen.total_tiles * screen.unit_minutes;
}

/**
 * A short phrase for the screen's period, or `null` when there is nothing to
 * say — an active screen with no window configured is simply open.
 */
export function timelineLabel(screen: Lifecycle): string | null {
  switch (screen.phase) {
    case 'archived':
      return 'Archived';
    case 'ended':
      return screen.ends_on ? `Ended ${formatDayMonth(screen.ends_on)}` : 'Ended';
    case 'upcoming':
      return screen.starts_on ? `Starts ${formatDayMonth(screen.starts_on)}` : 'Not started';
    case 'active':
      return screen.ends_on ? `Ends ${formatDayMonth(screen.ends_on)}` : null;
  }
}

/**
 * Why logging is refused, phrased for the member rather than the API.
 *
 * `null` when logging is permitted.
 */
export function blockedReason(screen: Lifecycle): string | null {
  switch (screen.phase) {
    case 'active':
      return null;
    case 'upcoming':
      return screen.starts_on
        ? `Logging opens ${formatDayMonth(screen.starts_on)}.`
        : 'This screen has not started yet.';
    case 'ended':
      return 'This period has ended, so nothing further can be logged.';
    case 'archived':
      return 'This screen is archived and can no longer be changed.';
  }
}
