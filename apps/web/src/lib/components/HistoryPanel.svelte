<script lang="ts">
  /**
   * Your own entries on this screen.
   *
   * Only ever your own: there is no leaderboard and no per-member breakdown,
   * just the shared total shown on the board itself.
   */
  import type { EntrySummary } from '$lib/api/types';
  import { formatDate, formatMinutes } from '$lib/format';

  type Props = {
    entries: EntrySummary[] | null;
    archived: boolean;
    onedit: (entry: EntrySummary) => void;
    ondelete: (entry: EntrySummary) => void;
    onclose: () => void;
  };

  let { entries, archived, onedit, ondelete, onclose }: Props = $props();

  const total = $derived(
    (entries ?? []).reduce((sum, entry) => sum + entry.minutes, 0)
  );

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div
  class="backdrop"
  role="button"
  tabindex="-1"
  aria-label="Close"
  onclick={onclose}
  onkeydown={() => {}}
></div>

<aside class="drawer" aria-label="Your progress">
  <header>
    <div>
      <h2>Your progress</h2>
      <p class="muted small">{formatMinutes(total)} across {entries?.length ?? 0} entries</p>
    </div>
    <button class="btn btn-sm" onclick={onclose}>Close</button>
  </header>

  {#if !entries}
    <p class="muted">Loading…</p>
  {:else if entries.length === 0}
    <p class="muted small">You have not logged anything here yet.</p>
  {:else}
    <ul>
      {#each entries as entry (entry.id)}
        <li>
          <div class="entry">
            <strong>{formatMinutes(entry.minutes)}</strong>
            <span class="muted small">{formatDate(entry.occurred_at)}</span>
            {#if entry.edited_at}<span class="muted small">· edited</span>{/if}
          </div>
          {#if entry.note}<p class="note">{entry.note}</p>{/if}
          {#if !archived}
            <div class="row actions">
              <button class="btn btn-sm" onclick={() => onedit(entry)}>Edit</button>
              <button class="btn btn-sm btn-danger" onclick={() => ondelete(entry)}>Delete</button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: rgb(0 0 0 / 0.5);
    border: none;
    padding: 0;
  }

  .drawer {
    position: fixed;
    z-index: 21;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(380px, 100vw);
    overflow-y: auto;
    padding: 1.1rem 1rem calc(1.1rem + env(safe-area-inset-bottom));
    background: var(--surface);
    color: var(--text);
    border-left: 1px solid var(--border);
    box-shadow: -12px 0 40px rgb(0 0 0 / 0.25);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  header h2 { margin: 0; }
  header p { margin: 0; }

  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.6rem; }

  li {
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
  }

  .entry { display: flex; align-items: baseline; gap: 0.45rem; flex-wrap: wrap; }
  .note { margin: 0.35rem 0 0; font-size: 0.9rem; overflow-wrap: anywhere; }
  .actions { margin-top: 0.55rem; }
</style>
