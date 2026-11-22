<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { setUnauthorizedHandler } from '$lib/api/client';
  import { auth } from '$lib/stores/auth.svelte';
  import '../app.css';

  let { children } = $props();

  /**
   * Pages reachable without a session. A predicate rather than a list: an
   * invite's path carries its token, so it cannot be matched by equality.
   */
  const isPublicPath = (path: string) => path === '/login' || path.startsWith('/join/');

  // The board occupies the whole viewport, so it gets no application chrome.
  const isBoard = $derived(page.url.pathname.startsWith('/s/'));
  const isPublic = $derived(isPublicPath(page.url.pathname));

  setUnauthorizedHandler(() => {
    auth.clear();
    if (!isPublicPath(location.pathname)) {
      goto(`/login?next=${encodeURIComponent(location.pathname + location.search)}`);
    }
  });

  auth.hydrate();

  $effect(() => {
    if (auth.loading) return;
    if (!auth.isAuthenticated && !isPublic) {
      goto(`/login?next=${encodeURIComponent(page.url.pathname + page.url.search)}`);
    }
  });

  async function signOut() {
    await auth.logout();
    goto('/login');
  }
</script>

{#if auth.loading}
  <div class="boot" role="status">Loading…</div>
{:else if !auth.isAuthenticated && !isPublic}
  <div class="boot" role="status">Redirecting…</div>
{:else}
  {#if !isBoard && !isPublic}
    <header class="nav">
      <div class="nav-inner">
        <a class="brand" href="/">Grid Reveal</a>
        <nav class="links">
          <a href="/">Screens</a>
          {#if auth.isAdmin}
            <a href="/admin">Admin</a>
          {/if}
          <a href="/account">Account</a>
          <button class="btn btn-sm" onclick={signOut}>Sign out</button>
        </nav>
      </div>
    </header>
  {/if}

  {@render children()}
{/if}

<style>
  .boot {
    display: grid;
    place-items: center;
    min-height: 100dvh;
    color: var(--text-muted);
  }

  .nav {
    position: sticky;
    top: 0;
    z-index: 10;
    background: color-mix(in srgb, var(--surface) 88%, transparent);
    backdrop-filter: blur(8px);
    border-bottom: 1px solid var(--border);
  }

  .nav-inner {
    max-width: 880px;
    margin: 0 auto;
    padding: 0.6rem 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .brand {
    font-weight: 650;
    color: var(--text);
    text-decoration: none;
    letter-spacing: -0.01em;
  }

  .links {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    font-size: 0.9rem;
  }

  .links a {
    color: var(--text-muted);
    text-decoration: none;
  }
  .links a:hover { color: var(--text); }

  @media (max-width: 480px) {
    .links { gap: 0.6rem; font-size: 0.85rem; }
  }
</style>
