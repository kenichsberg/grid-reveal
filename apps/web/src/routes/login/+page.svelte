<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError } from '$lib/api/client';
  import { pageTitle, site } from '$lib/stores/site.svelte';
  import { auth } from '$lib/stores/auth.svelte';

  let username = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    error = '';
    busy = true;
    try {
      await auth.login(username, password);
      const next = page.url.searchParams.get('next');
      goto(next && next.startsWith('/') ? next : '/');
    } catch (e) {
      // The API returns one error for both a wrong password and an unknown
      // account; the wording here must not imply otherwise.
      error =
        e instanceof ApiError && e.isUnauthorized
          ? 'Incorrect username or password.'
          : e instanceof ApiError
            ? e.message
            : 'Something went wrong.';
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>{pageTitle('Sign in')}</title></svelte:head>

<main class="wrap">
  <form class="card" onsubmit={submit}>
    <h1>{site.name}</h1>
    <p class="muted small">Sign in to see your screens.</p>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="field">
      <label for="username">Username</label>
      <input id="username" type="text" bind:value={username}
        autocomplete="username" autocapitalize="none" autocorrect="off" required />
    </div>

    <div class="field">
      <label for="password">Password</label>
      <input id="password" type="password" bind:value={password}
        autocomplete="current-password" required />
    </div>

    <button class="btn btn-primary full" type="submit" disabled={busy}>
      {busy ? 'Signing in…' : 'Sign in'}
    </button>

    <p class="muted small note">
      Accounts are created by an administrator. If you have forgotten your
      password, ask them to reset it.
    </p>
  </form>
</main>

<style>
  .wrap { min-height: 100dvh; display: grid; place-items: center; padding: 1.5rem 16px; }
  form { width: 100%; max-width: 360px; }
  .full { width: 100%; }
  .note { margin: 0.9rem 0 0; }
</style>
