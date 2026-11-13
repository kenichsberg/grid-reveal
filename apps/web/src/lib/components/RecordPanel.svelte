<script lang="ts">
  /**
   * Your own entries on this screen.
   *
   * Only ever your own: there is no leaderboard and no per-member breakdown,
   * just the shared total shown on the board itself. The member list is
   * public; what each member logged is not.
   *
   * Tab content, not an overlay — the shell places it and the tab bar is the
   * way out, so this owns no backdrop, no close button and no Escape key.
   */
  import type { EntrySummary } from '$lib/api/types';
  import { formatDate, formatMinutes } from '$lib/format';

  type Props = {
    entries: EntrySummary[] | null;
    /** False once the period has closed: the list stays, the controls go. */
    canEdit: boolean;
    onedit: (entry: EntrySummary) => void;
    ondelete: (entry: EntrySummary) => void;
  };

  let { entries, canEdit, onedit, ondelete }: Props = $props();

  const total = $derived(
    (entries ?? []).reduce((sum, entry) => sum + entry.minutes, 0)
  );
</script>

<div class="body">
  <header>
    <h2>Your progress</h2>
    <p class="muted small">{formatMinutes(total)} across {entries?.length ?? 0} entries</p>
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
          {#if canEdit}
            <div class="row actions">
              <button class="btn btn-sm" onclick={() => onedit(entry)}>Edit</button>
              <button class="btn btn-sm btn-danger" onclick={() => ondelete(entry)}>Delete</button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .body {
    /* Padding lives here rather than on the shell's .panel so the scrollbar
       tracks the panel edge, not the text column. */
    padding: 1.1rem 1rem 2rem;
  }

  header {
    margin-bottom: 1rem;
  }
  header h2 {
    margin: 0;
    font-size: 1.05rem;
  }
  header p { margin: 0.15rem 0 0; }

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
