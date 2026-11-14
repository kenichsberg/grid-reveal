<script lang="ts">
  /**
   * Goals: free-text bullets any member may add.
   *
   * Tab content, not an overlay. It used to dock to the bottom edge of the
   * board and expand over the picture; the shell now gives it a place of its
   * own, so it no longer positions itself, owns no open/closed state, and is
   * styled as a document on `--surface` rather than as glass over black.
   */
  import { api } from '$lib/api/client';
  import type { GoalSummary } from '$lib/api/types';

  type Props = {
    slug: string;
    goals: GoalSummary[];
    /** Administrators may delete any goal; authors may delete their own. */
    viewerIsAdmin: boolean;
    /** False once the period has closed: goals freeze with everything else. */
    canEdit: boolean;
    onchange: (goals: GoalSummary[]) => void;
  };

  let { slug, goals, viewerIsAdmin, canEdit, onchange }: Props = $props();

  const MAX_LEN = 200;
  const MAX_GOALS = 50;

  let text = $state('');
  let busy = $state(false);
  let error = $state('');

  const remaining = $derived(MAX_LEN - text.length);
  const atCapacity = $derived(goals.length >= MAX_GOALS);

  async function add(event: SubmitEvent) {
    event.preventDefault();
    const trimmed = text.trim();
    if (!trimmed || busy) return;

    busy = true;
    error = '';
    try {
      const created = await api.createGoal(slug, { text: trimmed });
      onchange([...goals, created]);
      text = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not add that goal.';
    } finally {
      busy = false;
    }
  }

  async function remove(goal: GoalSummary) {
    error = '';
    try {
      await api.deleteGoal(goal.id);
      onchange(goals.filter((g) => g.id !== goal.id));
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not delete that goal.';
    }
  }

  function canDelete(goal: GoalSummary) {
    return canEdit && (viewerIsAdmin || goal.is_mine);
  }
</script>

<div class="body">
  <header>
    <h2>Goals</h2>
    <p class="muted small">{goals.length} {goals.length === 1 ? 'goal' : 'goals'}</p>
  </header>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if goals.length === 0}
    <p class="muted small empty">No goals yet.</p>
  {:else}
    <ul>
      {#each goals as goal (goal.id)}
        <li>
          <span class="bullet" aria-hidden="true">•</span>
          <span class="text">{goal.text}</span>
          {#if goal.author_username}
            <!-- Authorship is disclosed to administrators only. -->
            <span class="author muted small">{goal.author_username}</span>
          {/if}
          {#if canDelete(goal)}
            <button
              class="remove"
              onclick={() => remove(goal)}
              aria-label={`Delete goal: ${goal.text}`}
            >
              ×
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if canEdit}
    {#if atCapacity}
      <p class="muted small">
        This screen has the maximum of {MAX_GOALS} goals.
      </p>
    {:else}
      <form class="add" onsubmit={add}>
        <input
          type="text"
          bind:value={text}
          maxlength={MAX_LEN}
          placeholder="Share my goal…"
          aria-label="New goal"
          enterkeyhint="done"
        />
        <button class="btn btn-sm btn-primary" type="submit" disabled={busy || !text.trim()}>
          Share
        </button>
      </form>
      {#if text.length > MAX_LEN - 40}
        <p class="muted small counter">{remaining} characters left</p>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .body { padding: 1.1rem 1rem 2rem; }

  header { margin-bottom: 1rem; }
  header h2 { margin: 0; font-size: 1.05rem; }
  header p { margin: 0.15rem 0 0; }

  ul { list-style: none; margin: 0 0 0.9rem; padding: 0; display: grid; gap: 0.45rem; }

  li {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: baseline;
    gap: 0.5rem;
    padding: 0.5rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
  }

  .bullet { opacity: 0.55; }
  .text { overflow-wrap: anywhere; }
  .author { white-space: nowrap; }

  .remove {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 1.15rem;
    line-height: 1;
    padding: 0 0.25rem;
    cursor: pointer;
  }
  .remove:hover { color: var(--danger); }

  .empty { margin: 0 0 0.9rem; }

  .add { display: flex; gap: 0.5rem; }
  .add input { flex: 1; }

  .counter { margin: 0.35rem 0 0; }
</style>
