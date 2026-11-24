<script lang="ts">
  import { api } from '$lib/api/client';
  import { pageTitle } from '$lib/stores/site.svelte';
  import { auth } from '$lib/stores/auth.svelte';

  let current = $state('');
  let next = $state('');
  let confirm = $state('');

  let busy = $state(false);
  let error = $state('');
  let done = $state('');

  const mismatch = $derived(confirm !== '' && next !== confirm);
  const tooShort = $derived(next !== '' && next.length < 8);
  const valid = $derived(current !== '' && next.length >= 8 && next === confirm);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!valid || busy) return;

    busy = true;
    error = '';
    done = '';
    try {
      await api.changePassword({ current_password: current, new_password: next });
      done = 'Password changed. Your other sessions have been signed out.';
      current = '';
      next = '';
      confirm = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not change your password.';
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>{pageTitle('Account')}</title></svelte:head>

<main class="container narrow">
  <h1>Account</h1>

  <div class="card details">
    <div class="spread">
      <div>
        <strong>{auth.user?.username}</strong>
        <p class="muted small">{auth.isAdmin ? 'Administrator' : 'Participant'}</p>
      </div>
    </div>
  </div>

  <form class="card" onsubmit={submit}>
    <h2>Change password</h2>

    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if done}<p class="done" role="status">{done}</p>{/if}

    <div class="field">
      <label for="current">Current password</label>
      <input id="current" type="password" bind:value={current} autocomplete="current-password" />
    </div>

    <div class="field">
      <label for="next">New password</label>
      <input id="next" type="password" bind:value={next} autocomplete="new-password" />
      {#if tooShort}<p class="hint">At least 8 characters.</p>{/if}
    </div>

    <div class="field">
      <label for="confirm">Confirm new password</label>
      <input id="confirm" type="password" bind:value={confirm} autocomplete="new-password" />
      {#if mismatch}<p class="hint">These do not match.</p>{/if}
    </div>

    <button class="btn btn-primary" type="submit" disabled={!valid || busy}>
      {busy ? 'Saving…' : 'Change password'}
    </button>
  </form>
</main>

<style>
  .narrow { max-width: 440px; }
  .details { margin-bottom: 1rem; }
  .details p { margin: 0; }
  .hint { margin: 0.3rem 0 0; font-size: 0.82rem; color: var(--danger); }
  .done {
    padding: 0.6rem 0.75rem;
    border-radius: 8px;
    background: color-mix(in srgb, var(--success) 14%, transparent);
    color: var(--success);
    border: 1px solid color-mix(in srgb, var(--success) 35%, transparent);
    font-size: 0.9rem;
  }
</style>
