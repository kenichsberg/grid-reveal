<script lang="ts">
  import { api, pictureUrl } from '$lib/api/client';
  import type { ScreenList, ScreenSummary } from '$lib/api/types';
  import { auth } from '$lib/stores/auth.svelte';
  import { percent } from '$lib/format';
  import { timelineLabel } from '$lib/screen';

  let screens = $state<ScreenList | null>(null);
  let error = $state('');

  async function load() {
    try {
      screens = await api.listScreens();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load your screens.';
    }
  }

  load();

  const hasAny = $derived(
    (screens?.active.length ?? 0) + (screens?.archived.length ?? 0) > 0
  );
</script>

<svelte:head><title>Screens · Grid Reveal</title></svelte:head>

<main class="container">
  <div class="spread head">
    <h1>Your screens</h1>
    {#if auth.isAdmin}
      <a class="btn btn-primary btn-sm" href="/admin/screens/new">New screen</a>
    {/if}
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {:else if !screens}
    <p class="muted">Loading…</p>
  {:else if !hasAny}
    <div class="card empty">
      <p>You are not on any screens yet.</p>
      <p class="muted small">
        An administrator adds you to a screen; it will appear here once they do.
      </p>
    </div>
  {:else}
    {#if screens.active.length > 0}
      <ul class="grid">
        {#each screens.active as screen (screen.slug)}
          {@render card(screen)}
        {/each}
      </ul>
    {/if}

    {#if screens.archived.length > 0}
      <h2 class="section">Archived</h2>
      <ul class="grid archived">
        {#each screens.archived as screen (screen.slug)}
          {@render card(screen)}
        {/each}
      </ul>
    {/if}
  {/if}
</main>

{#snippet card(screen: ScreenSummary)}
  {@const pct = percent(screen.revealed_count, screen.total_tiles)}
  <!-- `percent` rounds, so 249 of 250 tiles prints 100% beside a board that is
       still covered. The figure in the card's mouth is the one place that
       would read as a finished screen with its picture missing, so it holds at
       99 until the server sends the picture. -->
  {@const headline = screen.picture_url ? 100 : Math.min(99, pct)}
  <li>
    <a class="screen" href={`/s/${screen.slug}`}>
      <div class="thumb">
        <!-- The picture arrives only on a finished board; until then the card
             is a blank tile, because a dimmed photograph is still the
             photograph and there would be nothing left to uncover. -->
        {#if screen.picture_url}
          <img src={pictureUrl(screen.picture_url)} alt="" loading="lazy" />
        {:else}
          <span class="pct">{headline}%</span>
        {/if}
        <span class="badge">{timelineLabel(screen)}</span>
      </div>
      <div class="meta">
        <strong>{screen.title}</strong>
        <!-- The percentage and the tile count are already the card's largest
             element, so the one line under the title says who is behind them
             instead of repeating them. -->
        <span class="muted small">
          {screen.participant_count}
          {screen.participant_count === 1 ? 'participant' : 'participants'}
        </span>
        <div class="bar" aria-hidden="true"><div class="fill" style={`width: ${pct}%`}></div></div>
      </div>
    </a>
  </li>
{/snippet}

<style>
  .head { margin-bottom: 1rem; }
  .section { margin: 2rem 0 0.75rem; color: var(--text-muted); }

  .empty { text-align: center; padding: 2rem 1rem; }
  .empty p { margin: 0 0 0.35rem; }

  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 1rem;
  }

  .archived { opacity: 0.72; }

  .screen {
    display: block;
    text-decoration: none;
    color: inherit;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    box-shadow: var(--shadow);
    transition: transform 120ms ease, border-color 120ms ease;
  }
  .screen:hover { transform: translateY(-2px); border-color: var(--accent); }

  /* The same gray `BoardCanvas` paints covered tiles with, so an unfinished
     card reads as an unopened board rather than as a failed image. */
  .thumb { position: relative; aspect-ratio: 16 / 10; background: var(--tile); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }

  .pct {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 2rem;
    font-weight: 600;
    letter-spacing: -0.02em;
    color: #fff;
  }

  .badge {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: rgb(0 0 0 / 0.6);
    color: #fff;
    font-size: 0.7rem;
  }

  .meta { padding: 0.7rem 0.8rem 0.85rem; display: grid; gap: 0.25rem; }

  .bar {
    height: 4px;
    border-radius: 999px;
    background: var(--surface-2);
    overflow: hidden;
    margin-top: 0.2rem;
  }
  .fill { height: 100%; background: var(--accent); }
</style>
