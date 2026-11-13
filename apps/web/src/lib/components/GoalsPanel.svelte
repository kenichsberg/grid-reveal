<script lang="ts">
  /**
   * Goals, docked to the bottom edge of the board.
   *
   * An overlay rather than content below the fold: the board is specified to
   * fill the viewport, so the panel expands over the picture instead of
   * pushing it out of view or introducing page scroll.
   */
  import { api } from '$lib/api/client';
  import type { GoalSummary } from '$lib/api/types';

  type Props = {
    slug: string;
    goals: GoalSummary[];
    /** Administrators may delete any goal; authors may delete their own. */
    viewerIsAdmin: boolean;
    archived: boolean;
    onchange: (goals: GoalSummary[]) => void;
  };

  let { slug, goals, viewerIsAdmin, archived, onchange }: Props = $props();

  const MAX_LEN = 200;
  const MAX_GOALS = 50;

  let open = $state(false);
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
    return !archived && (viewerIsAdmin || goal.is_mine);
  }
</script>

<section class="panel" class:open aria-label="Goals">
  <button
    class="tab"
    onclick={() => (open = !open)}
    aria-expanded={open}
    aria-controls="goals-body"
  >
    <span class="chev" aria-hidden="true">{open ? '▾' : '▴'}</span>
    Goals
    <span class="count">{goals.length}</span>
  </button>

  {#if open}
    <div class="body" id="goals-body">
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

      {#if !archived}
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
              placeholder="Add a goal…"
              aria-label="New goal"
            />
            <button class="btn btn-sm btn-primary" type="submit" disabled={busy || !text.trim()}>
              Add
            </button>
          </form>
          {#if text.length > MAX_LEN - 40}
            <p class="muted small counter">{remaining} characters left</p>
          {/if}
        {/if}
      {/if}
    </div>
  {/if}
</section>

<style>
  .panel {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 3;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    pointer-events: none;
  }

  .tab,
  .body {
    pointer-events: auto;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0 0 0 12px;
    padding: 0.35rem 0.7rem;
    border: 1px solid rgb(255 255 255 / 0.18);
    border-bottom: none;
    border-radius: 8px 8px 0 0;
    background: rgb(0 0 0 / 0.62);
    backdrop-filter: blur(10px);
    color: #fff;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .tab:hover { background: rgb(0 0 0 / 0.75); }

  .chev { opacity: 0.8; font-size: 0.75rem; }

  .count {
    min-width: 1.3em;
    padding: 0 0.3em;
    border-radius: 999px;
    background: rgb(255 255 255 / 0.18);
    text-align: center;
    font-size: 0.75rem;
  }

  .body {
    width: 100%;
    max-height: 45dvh;
    overflow-y: auto;
    padding: 0.85rem 12px calc(0.85rem + env(safe-area-inset-bottom));
    background: rgb(0 0 0 / 0.72);
    backdrop-filter: blur(12px);
    border-top: 1px solid rgb(255 255 255 / 0.14);
    color: #fff;
  }

  .body .muted { color: rgb(255 255 255 / 0.62); }

  ul { list-style: none; margin: 0 0 0.75rem; padding: 0; display: grid; gap: 0.3rem; }

  li {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: baseline;
    gap: 0.5rem;
  }

  .bullet { opacity: 0.6; }
  .text { overflow-wrap: anywhere; }
  .author { white-space: nowrap; }

  .remove {
    background: none;
    border: none;
    color: rgb(255 255 255 / 0.55);
    font-size: 1.1rem;
    line-height: 1;
    padding: 0 0.25rem;
    cursor: pointer;
  }
  .remove:hover { color: #fff; }

  .empty { margin: 0 0 0.75rem; }

  .add { display: flex; gap: 0.5rem; }
  .add input {
    flex: 1;
    background: rgb(255 255 255 / 0.1);
    border-color: rgb(255 255 255 / 0.2);
    color: #fff;
  }
  .add input::placeholder { color: rgb(255 255 255 / 0.5); }

  .counter { margin: 0.35rem 0 0; }
</style>
