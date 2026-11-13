<script lang="ts">
  import { goto } from '$app/navigation';
  import { auth } from '$lib/stores/auth.svelte';

  // Administrative pages are also guarded server-side; this only avoids
  // showing a page that would fail every request it makes.
  $effect(() => {
    if (!auth.loading && !auth.isAdmin) goto('/');
  });
</script>

<svelte:head><title>Admin · Grid Reveal</title></svelte:head>

<main class="container">
  <h1>Administration</h1>

  <ul class="menu">
    <li>
      <a class="card" href="/admin/screens/new">
        <strong>Create a screen</strong>
        <span class="muted small">Pick a picture, set the target, choose the grid.</span>
      </a>
    </li>
    <li>
      <a class="card" href="/admin/screens">
        <strong>All screens</strong>
        <span class="muted small">Every screen, including ones you are not part of.</span>
      </a>
    </li>
    <li>
      <a class="card" href="/admin/pictures">
        <strong>Picture library</strong>
        <span class="muted small">Upload backgrounds and manage the shared library.</span>
      </a>
    </li>
    <li>
      <a class="card" href="/admin/users">
        <strong>Accounts</strong>
        <span class="muted small">Create accounts and reset passwords.</span>
      </a>
    </li>
  </ul>

  <p class="muted small note">
    You can open and manage any screen, whether or not you take part in it.
    Your own screen list shows only the ones you participate in.
  </p>
</main>

<style>
  .menu {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.75rem;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  }

  .menu a {
    display: grid;
    gap: 0.2rem;
    text-decoration: none;
    color: inherit;
    transition: border-color 120ms ease;
  }
  .menu a:hover { border-color: var(--accent); }

  .note { margin-top: 1.25rem; }
</style>
