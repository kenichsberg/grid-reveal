<script lang="ts">
  import { goto } from '$app/navigation';
  import { auth } from '$lib/stores/auth.svelte';
  import { pageTitle, site } from '$lib/stores/site.svelte';

  // Administrative pages are also guarded server-side; this only avoids
  // showing a page that would fail every request it makes.
  $effect(() => {
    if (!auth.loading && !auth.isAdmin) goto('/');
  });

  // Seeded from the name already in hand: the layout hydrated it before this
  // page rendered, so there is nothing to fetch.
  let appName = $state(site.name);
  let saving = $state(false);
  let error = $state('');
  let done = $state('');

  async function save(event: SubmitEvent) {
    event.preventDefault();
    error = '';
    done = '';
    saving = true;
    try {
      await site.rename(appName);
      // The store holds what the server stored, trimmed; show that rather
      // than what was typed.
      appName = site.name;
      done = `This installation is now called ${site.name}.`;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not save that name.';
    } finally {
      saving = false;
    }
  }
</script>

<svelte:head><title>{pageTitle('Site settings')}</title></svelte:head>

<main class="container">
  <h1>Site settings</h1>
  <p class="muted small lead">
    These apply to the whole installation, not to one screen.
  </p>

  {#if done}<p class="done" role="status">{done}</p>{/if}

  <form class="card" onsubmit={save}>
    <h2>Name</h2>
    <p class="muted small">
      Shown in the header, on the sign-in and invitation pages, and in the
      browser tab.
    </p>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="field">
      <label for="app-name">Application name</label>
      <input id="app-name" type="text" bind:value={appName} maxlength="60" required />
    </div>

    <button class="btn btn-primary" type="submit" disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </form>
</main>

<style>
  .lead { margin: 0.2rem 0 1rem; }

  .card h2 { margin: 0 0 0.2rem; }
  .card p.muted { margin: 0 0 0.9rem; }

  .done {
    padding: 0.6rem 0.75rem;
    border-radius: 8px;
    background: color-mix(in srgb, var(--success) 14%, transparent);
    color: var(--success);
    border: 1px solid color-mix(in srgb, var(--success) 35%, transparent);
    font-size: 0.9rem;
    margin-bottom: 1rem;
  }
</style>
