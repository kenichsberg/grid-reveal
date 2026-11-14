<script lang="ts">
  /** Log progress: hours and minutes against a date, defaulting to today. */
  import { untrack } from 'svelte';
  import type { EntrySummary } from '$lib/api/types';
  import { splitMinutes, today } from '$lib/format';

  type Props = {
    /** When set, the form edits that entry instead of creating a new one. */
    editing?: EntrySummary | null;
    /**
     * What one logged session is called on this screen, as a bare noun. The
     * headings and the submit button are built from it, so "Daimoku" gives
     * "Add Daimoku" / "Edit Daimoku" and the default gives "Add entry".
     */
    noun?: string;
    busy: boolean;
    error: string;
    /** First day the screen's period covers, if it has one. */
    minDate?: string | null;
    /** Last day the screen's period covers, if it has one. */
    maxDate?: string | null;
    onsubmit: (payload: {
      hours: number;
      minutes: number;
      occurred_at: string;
      note: string | null;
    }) => void;
    oncancel: () => void;
  };

  let {
    editing = null,
    noun = 'entry',
    busy,
    error,
    minDate = null,
    maxDate = null,
    onsubmit,
    oncancel
  }: Props = $props();

  // The API refuses future dates and dates outside the screen's period; these
  // stop both at the widget so the member does not have to be told twice. The
  // end of the window can be in the past, so the later bound is whichever of
  // the two comes first.
  const latest = $derived(maxDate && maxDate < today() ? maxDate : today());

  // Seeded from the entry being edited, deliberately once: the parent keys
  // this component on the entry id, so switching targets remounts it rather
  // than mutating fields under the user's cursor.
  const initial = untrack(() =>
    editing
      ? { ...splitMinutes(editing.minutes), date: editing.occurred_at, note: editing.note ?? '' }
      : { hours: 0, minutes: 0, date: today(), note: '' }
  );

  let hours = $state(initial.hours);
  let minutes = $state(initial.minutes);
  let date = $state(initial.date);
  let note = $state(initial.note);

  const total = $derived(hours * 60 + minutes);
  const valid = $derived(total >= 1 && hours >= 0 && hours <= 23 && minutes >= 0 && minutes <= 59);

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!valid || busy) return;
    onsubmit({
      hours,
      minutes,
      occurred_at: date,
      note: note.trim() === '' ? null : note.trim()
    });
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') oncancel();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div
  class="backdrop"
  role="button"
  tabindex="-1"
  aria-label="Close"
  onclick={oncancel}
  onkeydown={() => {}}
></div>

<div class="sheet" role="dialog" aria-modal="true" aria-labelledby="entry-title">
  <form onsubmit={submit}>
    <h2 id="entry-title">{editing ? `Edit ${noun}` : `Add ${noun}`}</h2>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="duration">
      <div class="field">
        <label for="hours">Hours</label>
        <input id="hours" type="number" min="0" max="23" bind:value={hours} inputmode="numeric" />
      </div>
      <div class="field">
        <label for="minutes">Minutes</label>
        <input id="minutes" type="number" min="0" max="59" bind:value={minutes} inputmode="numeric" />
      </div>
    </div>

    <div class="field">
      <label for="date">Date</label>
      <input id="date" type="date" bind:value={date} min={minDate ?? undefined} max={latest} />
    </div>

    <div class="field">
      <label for="note">Note <span class="muted small">(optional)</span></label>
      <input id="note" type="text" bind:value={note} maxlength="500" />
    </div>

    <div class="actions">
      <button class="btn" type="button" onclick={oncancel} disabled={busy}>Cancel</button>
      <button class="btn btn-primary" type="submit" disabled={!valid || busy}>
        {busy ? 'Saving…' : editing ? 'Save' : `Add ${noun}`}
      </button>
    </div>

    {#if total < 1}
      <p class="muted small hint">Log at least one minute.</p>
    {/if}
  </form>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: rgb(0 0 0 / 0.5);
    border: none;
    padding: 0;
  }

  .sheet {
    position: fixed;
    z-index: 21;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: min(400px, calc(100vw - 32px));
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: 0 20px 60px rgb(0 0 0 / 0.35);
    padding: 1.1rem;
  }

  .duration { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .hint { margin: 0.5rem 0 0; text-align: right; }

  @media (max-width: 480px) {
    .sheet {
      top: auto;
      bottom: 0;
      transform: translateX(-50%);
      width: 100vw;
      border-radius: 14px 14px 0 0;
      padding-bottom: calc(1.1rem + env(safe-area-inset-bottom));
    }
  }
</style>
