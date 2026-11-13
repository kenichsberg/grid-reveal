<script lang="ts">
  import { page } from '$app/state';
  import { api, ApiError, pictureUrl } from '$lib/api/client';
  import type { EntrySummary, GoalSummary, ScreenDetail } from '$lib/api/types';
  import BoardCanvas from '$lib/components/BoardCanvas.svelte';
  import Celebration from '$lib/components/Celebration.svelte';
  import EntryForm from '$lib/components/EntryForm.svelte';
  import GoalsPanel from '$lib/components/GoalsPanel.svelte';
  import HistoryPanel from '$lib/components/HistoryPanel.svelte';
  import { formatMinutes, percent } from '$lib/format';

  const slug = $derived(page.params.slug!);

  let screen = $state<ScreenDetail | null>(null);
  let goals = $state<GoalSummary[]>([]);
  let entries = $state<EntrySummary[] | null>(null);
  let loadError = $state('');

  let board = $state<ReturnType<typeof BoardCanvas> | null>(null);

  let showEntryForm = $state(false);
  let editing = $state<EntrySummary | null>(null);
  let formBusy = $state(false);
  let formError = $state('');

  let showHistory = $state(false);
  let celebrationDismissed = $state(false);

  const pct = $derived(
    screen ? percent(screen.revealed_count, screen.total_tiles) : 0
  );

  // The celebration shows for everyone on every visit, until dismissed for
  // this visit.
  const showCelebration = $derived(
    screen !== null && screen.completed && !celebrationDismissed
  );

  async function load() {
    loadError = '';
    try {
      const [detail, goalList] = await Promise.all([
        api.getScreen(slug),
        api.listGoals(slug)
      ]);
      screen = detail;
      goals = goalList;
    } catch (e) {
      // A screen that is not yours is indistinguishable from one that does not
      // exist, and the message must not suggest otherwise.
      loadError =
        e instanceof ApiError && e.isNotFound
          ? 'This screen does not exist, or you are not a participant on it.'
          : e instanceof Error
            ? e.message
            : 'Could not load this screen.';
    }
  }

  $effect(() => {
    void slug;
    screen = null;
    entries = null;
    celebrationDismissed = false;
    load();
  });

  async function loadEntries() {
    try {
      entries = await api.listEntries(slug);
    } catch (e) {
      console.error(e);
      entries = [];
    }
  }

  function openHistory() {
    showHistory = true;
    if (entries === null) loadEntries();
  }

  /**
   * Submits progress, then applies the result in two steps.
   *
   * Tiles that other members revealed while this page was open are snapped in
   * without animation; only the tiles this submission earned are animated. The
   * animation therefore always means "this is what you just did".
   */
  async function submitEntry(payload: {
    hours: number;
    minutes: number;
    occurred_at: string;
    note: string | null;
  }) {
    if (!screen) return;
    formBusy = true;
    formError = '';

    try {
      if (editing) {
        await api.updateEntry(editing.id, payload);
        // An edit can move the board in either direction, so re-read rather
        // than trying to animate a delta.
        await Promise.all([load(), loadEntries()]);
      } else {
        const alreadyShown = new Set(screen.revealed_tiles);
        const response = await api.createEntry(slug, payload);

        const yours = new Set(response.your_tiles);
        const othersTiles = response.revealed_tiles.filter(
          (tile) => !alreadyShown.has(tile) && !yours.has(tile)
        );

        // Order matters: mark the animation before the new revealed list
        // reaches the canvas, or the tiles would settle instantly.
        board?.snapTo(othersTiles);
        board?.animateReveal(response.your_tiles);

        screen = {
          ...screen,
          total_minutes: response.total_minutes,
          revealed_count: response.revealed_count,
          revealed_tiles: response.revealed_tiles,
          completed: response.completed
        };

        if (entries !== null) entries = [response.entry, ...entries];
      }

      showEntryForm = false;
      editing = null;
    } catch (e) {
      formError = e instanceof Error ? e.message : 'Could not save that entry.';
    } finally {
      formBusy = false;
    }
  }

  async function deleteEntry(entry: EntrySummary) {
    if (!confirm(`Delete this ${formatMinutes(entry.minutes)} entry? The board will rewind.`)) {
      return;
    }
    try {
      await api.deleteEntry(entry.id);
      await Promise.all([load(), loadEntries()]);
    } catch (e) {
      console.error(e);
    }
  }

  function startEdit(entry: EntrySummary) {
    editing = entry;
    showHistory = false;
    showEntryForm = true;
  }

  function closeForm() {
    showEntryForm = false;
    editing = null;
    formError = '';
  }
</script>

<svelte:head>
  <title>{screen?.title ?? 'Screen'} · Grid Reveal</title>
</svelte:head>

{#if loadError}
  <main class="container">
    <p class="error" role="alert">{loadError}</p>
    <p><a href="/">Back to your screens</a></p>
  </main>
{:else if !screen}
  <main class="container"><p class="muted">Loading…</p></main>
{:else}
  <div class="screen">
    <BoardCanvas
      bind:this={board}
      src={pictureUrl(screen.picture_url)}
      pictureWidth={screen.picture_width}
      pictureHeight={screen.picture_height}
      cols={screen.cols}
      rows={screen.rows}
      revealed={screen.revealed_tiles}
    />

    <header class="bar top">
      <div class="left">
        <a class="back" href="/" aria-label="Back to your screens">‹</a>
        <div>
          <h1>{screen.title}</h1>
          <p class="sub">
            {screen.participant_count}
            {screen.participant_count === 1 ? 'participant' : 'participants'}
            {#if screen.archived}· Archived{/if}
          </p>
        </div>
      </div>
      {#if screen.viewer_is_admin}
        <a class="chip" href={`/admin/screens/${slug}`}>Manage</a>
      {/if}
    </header>

    <footer class="bar bottom">
      <div class="progress">
        <div class="figures">
          <strong>{formatMinutes(screen.total_minutes)}</strong>
          <span class="sub">
            {screen.revealed_count}/{screen.total_tiles} tiles · {pct}%
          </span>
        </div>
        <div class="bar-track" aria-hidden="true">
          <div class="bar-fill" style={`width:${pct}%`}></div>
        </div>
      </div>

      <div class="row">
        <button class="chip" onclick={openHistory}>History</button>
        {#if !screen.archived}
          <button class="chip primary" onclick={() => (showEntryForm = true)}>
            Add progress
          </button>
        {/if}
      </div>
    </footer>

    <GoalsPanel
      {slug}
      {goals}
      viewerIsAdmin={screen.viewer_is_admin}
      archived={screen.archived}
      onchange={(next) => (goals = next)}
    />

    {#if showCelebration}
      <Celebration
        title={screen.title}
        totalMinutes={screen.total_minutes}
        ondismiss={() => (celebrationDismissed = true)}
      />
    {/if}
  </div>

  {#if showEntryForm}
    <!-- Keyed so editing a different entry remounts the form with its values. -->
    {#key editing?.id ?? 'new'}
      <EntryForm
        {editing}
        busy={formBusy}
        error={formError}
        onsubmit={submitEntry}
        oncancel={closeForm}
      />
    {/key}
  {/if}

  {#if showHistory}
    <HistoryPanel
      {entries}
      archived={screen.archived}
      onedit={startEdit}
      ondelete={deleteEntry}
      onclose={() => (showHistory = false)}
    />
  {/if}
{/if}

<style>
  .screen {
    position: fixed;
    inset: 0;
    overflow: hidden;
    background: #000;
  }

  .bar {
    position: absolute;
    left: 0;
    right: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.7rem 12px;
    color: #fff;
    pointer-events: none;
  }

  .bar :global(a),
  .bar button {
    pointer-events: auto;
  }

  .top {
    top: 0;
    padding-top: calc(0.7rem + env(safe-area-inset-top));
    background: linear-gradient(to bottom, rgb(0 0 0 / 0.55), transparent);
  }

  .bottom {
    bottom: 0;
    /* Clear of the goals tab, which docks to the same edge. */
    padding-bottom: calc(2.6rem + env(safe-area-inset-bottom));
    background: linear-gradient(to top, rgb(0 0 0 / 0.6), transparent);
    align-items: flex-end;
  }

  .left { display: flex; align-items: center; gap: 0.6rem; }

  .back {
    pointer-events: auto;
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.45);
    color: #fff;
    text-decoration: none;
    font-size: 1.3rem;
    line-height: 1;
  }
  .back:hover { background: rgb(0 0 0 / 0.65); }

  h1 {
    margin: 0;
    font-size: 1.05rem;
    text-shadow: 0 1px 3px rgb(0 0 0 / 0.5);
  }

  .sub {
    margin: 0;
    font-size: 0.8rem;
    color: rgb(255 255 255 / 0.78);
    text-shadow: 0 1px 3px rgb(0 0 0 / 0.5);
  }

  .progress { min-width: 0; flex: 1; max-width: 320px; }
  .figures { display: flex; align-items: baseline; gap: 0.45rem; flex-wrap: wrap; }
  .figures strong { text-shadow: 0 1px 3px rgb(0 0 0 / 0.5); }

  .bar-track {
    margin-top: 0.3rem;
    height: 4px;
    border-radius: 999px;
    background: rgb(255 255 255 / 0.25);
    overflow: hidden;
  }
  .bar-fill { height: 100%; background: #fff; transition: width 400ms ease; }

  .chip {
    pointer-events: auto;
    display: inline-flex;
    align-items: center;
    padding: 0.4rem 0.75rem;
    border-radius: 999px;
    border: 1px solid rgb(255 255 255 / 0.22);
    background: rgb(0 0 0 / 0.5);
    color: #fff;
    font-size: 0.85rem;
    text-decoration: none;
    cursor: pointer;
  }
  .chip:hover { background: rgb(0 0 0 / 0.68); }

  .chip.primary {
    background: var(--accent);
    border-color: transparent;
    color: #fff;
  }
  .chip.primary:hover { filter: brightness(1.1); }

  @media (max-width: 420px) {
    h1 { font-size: 0.95rem; }
    .progress { max-width: 180px; }
  }
</style>
