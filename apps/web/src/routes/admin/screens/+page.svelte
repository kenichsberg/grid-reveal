<script lang="ts">
  /**
   * The administrator screen directory: every screen, whether or not the
   * administrator participates in it.
   *
   * Deliberately separate from the personal screen list at `/`, which stays
   * member-scoped for everyone. "The screens I am part of" and "every screen I
   * can administer" are different questions.
   */
  import { api } from '$lib/api/client';
  import type { AdminScreenEntry } from '$lib/api/types';
  import { formatMinutes, percent } from '$lib/format';

  let screens = $state<AdminScreenEntry[] | null>(null);
  let error = $state('');
  let query = $state('');
  let showArchived = $state(false);

  async function load() {
    try {
      screens = await api.listAllScreens();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load the screen directory.';
    }
  }

  load();

  const visible = $derived.by(() => {
    const all = screens ?? [];
    const needle = query.trim().toLowerCase();
    return all
      .filter((screen) => showArchived || !screen.archived)
      .filter((screen) => needle === '' || screen.title.toLowerCase().includes(needle));
  });

  const archivedCount = $derived((screens ?? []).filter((s) => s.archived).length);
</script>

<svelte:head><title>All screens · Grid Reveal</title></svelte:head>

<main class="container">
  <div class="spread head">
    <h1>All screens</h1>
    <a class="btn btn-sm btn-primary" href="/admin/screens/new">New screen</a>
  </div>
  <p class="muted small lead">
    Every screen on this installation. Your own list shows only the ones you
    take part in.
  </p>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if !screens}
    <p class="muted">Loading…</p>
  {:else if screens.length === 0}
    <div class="card empty">
      <p class="muted">No screens have been created yet.</p>
    </div>
  {:else}
    <div class="controls row">
      <input type="text" bind:value={query} placeholder="Filter by title…" aria-label="Filter by title" />
      {#if archivedCount > 0}
        <label class="check">
          <input type="checkbox" bind:checked={showArchived} />
          <span class="small">Show archived ({archivedCount})</span>
        </label>
      {/if}
    </div>

    {#if visible.length === 0}
      <p class="muted small">Nothing matches that filter.</p>
    {:else}
      <ul class="list">
        {#each visible as screen (screen.slug)}
          {@const pct = percent(screen.revealed_count, screen.total_tiles)}
          <!-- No thumbnail: an administrator is a participant too, and a
               directory of pictures would spoil every board they are on. The
               picture is for choosing one and for a finished screen. -->
          <li class="card" class:archived={screen.archived}>
            <div class="meta">
              <div class="titles">
                <strong>{screen.title}</strong>
                {#if screen.archived}<span class="tag">archived</span>{/if}
                {#if !screen.viewer_is_member}
                  <!-- Administrators may open any screen; this marks the ones
                       they are not actually participating in. -->
                  <span class="tag quiet">not a participant</span>
                {/if}
              </div>
              <span class="muted small">
                {formatMinutes(screen.total_minutes)} ·
                {screen.revealed_count}/{screen.total_tiles} tiles · {pct}%
              </span>
              <div class="bar" aria-hidden="true">
                <div class="fill" style={`width:${pct}%`}></div>
              </div>
            </div>
            <div class="actions row">
              <a class="btn btn-sm" href={`/s/${screen.slug}`}>Open</a>
              <a class="btn btn-sm" href={`/admin/screens/${screen.slug}`}>Manage</a>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</main>

<style>
  .head { margin-bottom: 0; }
  .lead { margin: 0.2rem 0 1rem; }

  .controls { margin-bottom: 0.9rem; flex-wrap: wrap; }
  .controls input[type='text'] { max-width: 260px; }

  .check { display: flex; align-items: center; gap: 0.4rem; white-space: nowrap; }
  .check input { width: auto; }

  .empty { text-align: center; padding: 2rem; }

  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.6rem; }

  .list li {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 0.9rem;
    padding: 0.7rem;
  }

  .list li.archived { opacity: 0.68; }

  .meta { min-width: 0; display: grid; gap: 0.2rem; }
  .titles { display: flex; align-items: baseline; gap: 0.4rem; flex-wrap: wrap; }

  .tag {
    padding: 0.05rem 0.4rem;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 0.72rem;
    white-space: nowrap;
  }
  .tag.quiet { background: transparent; border: 1px solid var(--border); }

  .bar {
    height: 4px;
    border-radius: 999px;
    background: var(--surface-2);
    overflow: hidden;
    margin-top: 0.15rem;
  }
  .fill { height: 100%; background: var(--accent); }

  @media (max-width: 560px) {
    .list li { grid-template-columns: 1fr; }
    .actions { justify-content: flex-end; }
  }
</style>
