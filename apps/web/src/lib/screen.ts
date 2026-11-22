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
import { formatDayMonth, percent } from '$lib/format';

/**
 * The wording a screen falls back to when an administrator has set none.
 *
 * Neutral on purpose: the application is not about one practice, and the
 * particular word a group uses belongs in that screen's settings rather than
 * in this file.
 */
const DEFAULT_PROGRESS_TITLE = 'Progress';
const DEFAULT_ENTRY_NOUN = 'entry';

/** The fields these helpers read; keeps them usable from the list page too. */
type Lifecycle = {
  phase: Phase;
  starts_on: string;
  ends_on: string;
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
 * Progress towards the hour goal, as a whole percentage.
 *
 * Measured in logged time rather than in revealed tiles, so that the gauge,
 * the percentage in its mouth and the two hour figures beneath it are all the
 * same fraction. A tile-based figure would disagree with the hours printed
 * beside it whenever a tile is part-paid for, and two numbers that contradict
 * each other in one glance are worse than either alone.
 *
 * Clamped at 100: logged time may exceed the goal, but a gauge cannot.
 */
export function progressPercent(
  screen: Pick<ScreenDetail, 'total_minutes' | 'total_tiles' | 'unit_minutes'>
): number {
  return Math.min(100, percent(screen.total_minutes, goalMinutes(screen)));
}

/** The heading above the progress gauge, or the neutral default. */
export function progressTitle(screen: Pick<ScreenDetail, 'progress_title'>): string {
  return screen.progress_title?.trim() || DEFAULT_PROGRESS_TITLE;
}

/**
 * What one logged session is called on this screen.
 *
 * A bare noun, because the call sites build "Add …" and "Edit …" from it. The
 * default is lowercase so that "Add entry" reads as a sentence, while a
 * configured proper noun ("Daimoku") keeps its own capital.
 */
export function entryNoun(screen: Pick<ScreenDetail, 'entry_noun'>): string {
  return screen.entry_noun?.trim() || DEFAULT_ENTRY_NOUN;
}

/**
 * A short phrase for the screen's period. Every screen has one, so every phase
 * has a date to name.
 */
export function timelineLabel(screen: Lifecycle): string {
  switch (screen.phase) {
    case 'archived':
      return 'Archived';
    case 'ended':
      return `Ended ${formatDayMonth(screen.ends_on)}`;
    case 'upcoming':
      return `Starts ${formatDayMonth(screen.starts_on)}`;
    case 'active':
      return `Ends ${formatDayMonth(screen.ends_on)}`;
  }
}

/**
 * The screen's period as a span, for the line under the board's heading.
 *
 * Dates rather than {@link timelineLabel}'s phrase, because this sits as a
 * subtitle and is read as "what stretch of time is this board about" — not as
 * "what happens next".
 */
export function periodLabel(screen: Lifecycle): string {
  const span = `${formatDayMonth(screen.starts_on)} – ${formatDayMonth(screen.ends_on)}`;

  // An archived screen says so: that is the one phase a bare pair of dates
  // would not convey.
  return screen.phase === 'archived' ? `${span} · Archived` : span;
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
      return `Logging opens ${formatDayMonth(screen.starts_on)}.`;
    case 'ended':
      return 'This period has ended, so nothing further can be logged.';
    case 'archived':
      return 'This screen is archived and can no longer be changed.';
  }
}
