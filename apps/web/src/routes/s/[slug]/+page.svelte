<script lang="ts">
  /**
   * The board, and the three things that surround it.
   *
   * The chrome is RESERVED, not floated over the picture: the shell is a grid
   * whose middle track is the board, so the letterbox a photograph would have
   * produced anyway becomes the title band and the tab bar. BoardCanvas sizes
   * itself from a ResizeObserver on its own element, so giving it a smaller box
   * is all it takes — none of the letterbox maths changes.
   */
  import { untrack } from 'svelte';
  import { page } from '$app/state';
  import { api, ApiError, pictureUrl } from '$lib/api/client';
  import type {
    EntrySummary,
    GoalSummary,
    MemberSummary,
    ScreenDetail
  } from '$lib/api/types';
  import BoardCanvas from '$lib/components/BoardCanvas.svelte';
  import Celebration from '$lib/components/Celebration.svelte';
  import EntryForm from '$lib/components/EntryForm.svelte';
  import GoalsPanel from '$lib/components/GoalsPanel.svelte';
  import MembersPanel from '$lib/components/MembersPanel.svelte';
  import RecordPanel from '$lib/components/RecordPanel.svelte';
  import { formatHours, formatMinutes, percent } from '$lib/format';
  import { blockedReason, canEditGoals, canLog, goalMinutes, timelineLabel } from '$lib/screen';

  const slug = $derived(page.params.slug!);

  const TABS = ['home', 'record', 'goals', 'members'] as const;
  type Tab = (typeof TABS)[number];

  const TAB_LABELS: Record<Tab, string> = {
    home: 'Home',
    record: 'Record',
    goals: 'Goals',
    members: 'Members'
  };

  /**
   * The open tab lives in the URL, not in component state.
   *
   * This is a tab bar on a phone, so the system back gesture has to close a
   * panel rather than leave the screen. Anything else and one careless swipe
   * loses the board. An unrecognised value falls back to Home without
   * rewriting the URL, because rewriting it invites a navigation loop.
   */
  const tab = $derived.by<Tab>(() => {
    const requested = page.url.searchParams.get('tab') ?? '';
    return (TABS as readonly string[]).includes(requested) ? (requested as Tab) : 'home';
  });

  let screen = $state<ScreenDetail | null>(null);
  let goals = $state<GoalSummary[]>([]);
  let entries = $state<EntrySummary[] | null>(null);
  let members = $state<MemberSummary[] | null>(null);
  let loadError = $state('');

  let board = $state<ReturnType<typeof BoardCanvas> | null>(null);

  let showEntryForm = $state(false);
  let editing = $state<EntrySummary | null>(null);
  let formBusy = $state(false);
  let formError = $state('');

  let celebrationDismissed = $state(false);

  const pct = $derived(
    screen ? percent(screen.revealed_count, screen.total_tiles) : 0
  );

  const writable = $derived(screen !== null && canLog(screen));
  const goalsWritable = $derived(screen !== null && canEditGoals(screen));
  const timeline = $derived(screen ? timelineLabel(screen) : null);
  const blocked = $derived(screen ? blockedReason(screen) : null);

  // The celebration shows for everyone on every visit, until dismissed for
  // this visit. A closed period does not retract it: the board was finished.
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

  // Guarded on the slug itself, not merely derived from it: switching tabs is a
  // query-string navigation, and without this the board would blank and refetch
  // on every tap of the tab bar.
  let loadedSlug = '';
  $effect(() => {
    if (slug === loadedSlug) return;
    loadedSlug = slug;
    screen = null;
    entries = null;
    members = null;
    celebrationDismissed = false;
    load();
  });

  let loadingEntries = false;
  async function loadEntries() {
    loadingEntries = true;
    try {
      entries = await api.listEntries(slug);
    } catch (e) {
      console.error(e);
      entries = [];
    } finally {
      loadingEntries = false;
    }
  }

  let loadingMembers = false;
  async function loadMembers() {
    loadingMembers = true;
    try {
      members = await api.listMembers(slug);
    } catch (e) {
      console.error(e);
      members = [];
    } finally {
      loadingMembers = false;
    }
  }

  // Both lists stay lazy; only the trigger moved from a button to a tab. The
  // untrack is load-bearing: each effect writes the same state it reads, so
  // without it the assignment would retrigger the effect.
  $effect(() => {
    if (tab !== 'record') return;
    void slug;
    untrack(() => {
      if (entries === null && !loadingEntries) void loadEntries();
    });
  });

  $effect(() => {
    if (tab !== 'members') return;
    void slug;
    untrack(() => {
      if (members === null && !loadingMembers) void loadMembers();
    });
  });

  function tabHref(next: Tab): string {
    return next === 'home' ? `/s/${slug}` : `/s/${slug}?tab=${next}`;
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

{#snippet tabIcon(item: Tab)}
  <!-- Line icons on a 24-unit grid, stroked in currentColor so they take the
       tab's active and inactive colours without a second rule. -->
  <svg
    class="tab-icon"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    {#if item === 'home'}
      <path d="M3 10.5 12 3l9 7.5" />
      <path d="M5.5 9.5V20h13V9.5" />
    {:else if item === 'record'}
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7v5.2l3.2 2" />
    {:else if item === 'goals'}
      <circle cx="12" cy="12" r="8.5" />
      <circle cx="12" cy="12" r="4" />
      <circle cx="12" cy="12" r="0.6" fill="currentColor" />
    {:else}
      <circle cx="9" cy="9" r="3.3" />
      <path d="M3.5 19.2c0-3 2.5-5 5.5-5s5.5 2 5.5 5" />
      <path d="M16 6.4a3.3 3.3 0 0 1 0 6.1" />
      <path d="M17.4 14.6c2 .6 3.3 2.3 3.3 4.6" />
    {/if}
  </svg>
{/snippet}

{#snippet addButton()}
  <button class="cta" onclick={() => (showEntryForm = true)}>Add Daimoku</button>
{/snippet}

{#if loadError}
  <main class="container">
    <p class="error" role="alert">{loadError}</p>
    <p><a href="/">Back to your screens</a></p>
  </main>
{:else if !screen}
  <main class="container"><p class="muted">Loading…</p></main>
{:else}
  <div class="shell" class:with-panel={tab !== 'home'}>
    <header class="band">
      <div class="band-head">
        <a class="back" href="/" aria-label="Back to your screens">‹</a>
        <h1>{screen.title}</h1>
        {#if screen.viewer_is_admin}
          <a class="chip" href={`/admin/screens/${slug}`}>Manage</a>
        {/if}
      </div>

      <div class="progress">
        <p class="progress-title">Daimoku Progress</p>

        <div class="stats">
          <p class="stat">
            <span class="label">current</span>
            <strong>{formatHours(screen.total_minutes)}<span class="unit">h</span></strong>
          </p>
          <p class="stat">
            <span class="label">goal</span>
            <strong>{formatHours(goalMinutes(screen))}<span class="unit">h</span></strong>
          </p>
          <p class="stat pct"><strong>{pct}%</strong></p>
        </div>

        <div class="meter" aria-hidden="true">
          <div class="meter-fill" style={`width:${pct}%`}></div>
        </div>
      </div>

      {#if timeline}<p class="timeline">{timeline}</p>{/if}


      <!-- Rendered from one snippet in two slots; CSS hides whichever slot the
           current width does not use, so the hidden copy is inert. -->
      {#if writable}
        <div class="rail-cta">{@render addButton()}</div>
      {/if}
    </header>

    <nav class="tabs" aria-label="Screen sections">
      {#each TABS as item (item)}
        <a
          href={tabHref(item)}
          aria-current={tab === item ? 'page' : undefined}
          data-sveltekit-noscroll
          data-sveltekit-replacestate={tab === 'home' ? undefined : 'true'}
        >
          {@render tabIcon(item)}
          <span class="tab-label">{TAB_LABELS[item]}</span>
        </a>
      {/each}
    </nav>

    <div class="stage">
      <BoardCanvas
        bind:this={board}
        src={pictureUrl(screen.picture_url)}
        pictureWidth={screen.picture_width}
        pictureHeight={screen.picture_height}
        cols={screen.cols}
        rows={screen.rows}
        revealed={screen.revealed_tiles}
      />

      {#if tab === 'home'}
        {#if writable}
          <div class="stage-cta">{@render addButton()}</div>
        {:else if blocked}
          <p class="stage-note">{blocked}</p>
        {/if}
      {/if}
    </div>

    {#if tab !== 'home'}
      <section class="panel" tabindex="-1" aria-label={tab === 'record' ? 'Your progress' : tab === 'goals' ? 'Goals' : 'Members'}>
        {#if tab === 'record'}
          <RecordPanel
            {entries}
            canEdit={writable}
            onedit={startEdit}
            ondelete={deleteEntry}
          />
        {:else if tab === 'goals'}
          <GoalsPanel
            {slug}
            {goals}
            viewerIsAdmin={screen.viewer_is_admin}
            canEdit={goalsWritable}
            onchange={(next) => (goals = next)}
          />
        {:else}
          <MembersPanel {members} participantCount={screen.participant_count} />
        {/if}
      </section>
    {/if}

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
        minDate={screen.starts_on}
        maxDate={screen.ends_on}
        onsubmit={submitEntry}
        oncancel={closeForm}
      />
    {/key}
  {/if}
{/if}

<style>
  .shell {
    position: fixed;
    /* Height rather than `inset: 0`: on iOS Safari `inset` resolves against the
       large viewport, which would put the tab bar under the browser toolbar.
       The shell never scrolls, so the toolbars stay expanded and 100dvh is
       stable — no resize churn behind the canvas. */
    inset: 0 0 auto 0;
    height: 100dvh;
    overflow: hidden;
    background: #000;
    color: var(--board-ink);

    display: grid;
    grid-template-columns: minmax(0, 1fr);
    /* The band sizes to its own content rather than to a fixed track: the
       title, the stats, the meter and the period line are a variable number of
       rows, and a guessed height clips them. --band-h is only a floor. */
    grid-template-rows:
      auto
      minmax(0, 1fr)
      calc(var(--tab-bar-h) + var(--safe-bottom));
    grid-template-areas: 'band' 'stage' 'tabs';
  }

  /* BoardCanvas's own `.board` is absolute/inset:0, so this element is both its
     containing block and its ResizeObserver target. The `minmax(0, 1fr)` above
     and these `min-*: 0` are what let the track actually shrink. */
  .stage {
    grid-area: stage;
    position: relative;
    min-width: 0;
    min-height: 0;
  }

  /* ——— band ——— */

  .band {
    grid-area: band;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.35rem;
    min-width: 0;
    min-height: calc(var(--band-h) + var(--safe-top));
    padding:
      calc(0.5rem + var(--safe-top))
      calc(12px + var(--safe-right))
      0.55rem
      calc(12px + var(--safe-left));
  }

  .band-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    min-height: 3rem;
  }

  .band-head h1 {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    letter-spacing: -0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .back {
    color: var(--board-ink);
    font-size: 2rem;
    line-height: 1;
    text-decoration: none;
    padding: 0 0.3rem;
  }

  .chip {
    padding: 0.25rem 0.6rem;
    border: 1px solid var(--board-line);
    border-radius: 999px;
    background: rgb(255 255 255 / 0.08);
    color: var(--board-ink);
    font-size: 0.78rem;
    text-decoration: none;
    white-space: nowrap;
  }

  .progress {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
  }

  .progress-title {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .stats {
    display: flex;
    align-items: baseline;
    gap: 1.4rem;
    min-width: 0;
  }

  .stat {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    margin: 0;
    min-width: 0;
  }

  .stat .label {
    color: var(--board-ink-dim);
    font-size: 0.9rem;
    letter-spacing: 0.02em;
    text-transform: lowercase;
  }

  .stat strong { font-size: 1.75rem; font-weight: 700; letter-spacing: -0.02em; }
  .stat .unit { font-size: 1rem; font-weight: 500; opacity: 0.85; }
  .pct { margin-left: auto; }
  .pct strong { font-size: 1.35rem; font-weight: 600; }

  .meter {
    height: 8px;
    border-radius: 999px;
    background: rgb(255 255 255 / 0.18);
    overflow: hidden;
  }

  .meter-fill {
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
    transition: width 400ms ease;
  }

  .timeline {
    margin: 0;
    color: var(--board-ink-dim);
    font-size: 0.75rem;
  }

  .rail-cta { display: none; }

  /* ——— tabs ——— */

  .tabs {
    grid-area: tabs;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    padding-bottom: var(--safe-bottom);
    border-top: 1px solid var(--board-line);
    background: #000;
  }

  .tabs a {
    display: grid;
    place-items: center;
    align-content: center;
    gap: 0.15rem;
    min-height: 44px;
    color: var(--board-ink-dim);
    text-decoration: none;
  }

  .tab-icon { width: 22px; height: 22px; }
  .tab-label { font-size: 0.68rem; letter-spacing: 0.01em; }

  .tabs a[aria-current='page'] {
    color: var(--board-ink);
    box-shadow: inset 0 2px 0 var(--accent);
  }

  /* ——— the primary action ——— */

  .stage-cta {
    position: absolute;
    left: 50%;
    bottom: 1.1rem;
    z-index: var(--z-stage-cta);
    transform: translateX(-50%);
  }

  .cta {
    padding: 0.85rem 1.8rem;
    border: none;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-text);
    font: inherit;
    font-size: 1.02rem;
    font-weight: 600;
    cursor: pointer;
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.5);
  }

  .stage-note {
    position: absolute;
    left: 50%;
    bottom: 1.1rem;
    z-index: var(--z-stage-cta);
    transform: translateX(-50%);
    max-width: min(26rem, calc(100% - 2rem));
    margin: 0;
    padding: 0.5rem 0.9rem;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.62);
    color: var(--board-ink-dim);
    font-size: 0.82rem;
    text-align: center;
  }

  /* ——— tab panels ——— */

  .panel {
    /* Narrow: covers the board, but the band and tab bar stay put, so the
       title and stats remain visible on every tab. */
    grid-area: stage;
    z-index: var(--z-panel);
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    background: var(--surface);
    color: var(--text);
    animation: panel-in 120ms ease-out;
  }

  .panel:focus { outline: none; }

  /* A CSS animation, not a Svelte transition: the global
     prefers-reduced-motion rule clamps this, and would not reach a
     JS-driven one. */
  @keyframes panel-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  /* ——— wide: rail, top tabs, stage, panel ——— */

  @media (min-width: 960px) {
    .shell {
      grid-template-columns: var(--rail-w) minmax(0, 1fr);
      grid-template-rows: calc(var(--topbar-h) + var(--safe-top)) minmax(0, 1fr);
      grid-template-areas:
        'band tabs'
        'band stage';
    }

    .shell.with-panel {
      grid-template-columns: var(--rail-w) minmax(0, 1fr) var(--panel-w);
      grid-template-areas:
        'band tabs  tabs'
        'band stage panel';
    }

    .band {
      justify-content: flex-start;
      gap: 0.8rem;
      overflow-y: auto;
      padding:
        calc(1rem + var(--safe-top))
        1rem
        calc(1rem + var(--safe-bottom))
        calc(1rem + var(--safe-left));
      border-right: 1px solid var(--board-line);
    }

    .band-head { flex-wrap: wrap; }
    .band-head h1 {
      flex-basis: 100%;
      order: 2;
      font-size: 1.2rem;
      white-space: normal;
      overflow: visible;
    }
    .chip { order: 1; margin-left: auto; }

    .stats { flex-direction: column; align-items: flex-start; gap: 0.5rem; }
    .pct { margin-left: 0; }

    /* With a cursor there is no thumb to reach, so the primary action belongs
       at the top of the rail rather than floating over the artwork. */
    .rail-cta { display: block; }
    .cta { width: 100%; }
    .stage-cta { display: none; }

    .tabs {
      grid-template-columns: repeat(4, max-content);
      justify-content: start;
      padding: 0 0.75rem;
      border-top: none;
      border-bottom: 1px solid var(--board-line);
    }
    /* Wide: the icon sits beside its label rather than above it, so the strip
       stays one row tall. */
    .tabs a { grid-auto-flow: column; gap: 0.4rem; padding: 0 0.9rem; }
    .tabs .tab-label { font-size: 0.85rem; }
    .tabs .tab-icon { width: 18px; height: 18px; }

    .panel {
      /* Its own column: nothing to stack over, and no width transition — the
         canvas would reallocate and fully redraw on every animated frame. */
      grid-area: panel;
      z-index: auto;
      border-left: 1px solid var(--border);
      animation: none;
    }
  }
</style>
