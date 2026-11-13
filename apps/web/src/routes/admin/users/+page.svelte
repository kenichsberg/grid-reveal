<script lang="ts">
  import { api } from '$lib/api/client';
  import type { UserSummary } from '$lib/api/types';

  let users = $state<UserSummary[] | null>(null);
  let error = $state('');

  // Create
  let username = $state('');
  let password = $state('');
  let isAdmin = $state(false);
  let creating = $state(false);
  let createError = $state('');

  // Reset
  let resetTarget = $state<UserSummary | null>(null);
  let resetPassword = $state('');
  let resetting = $state(false);
  let resetError = $state('');
  let resetDone = $state('');

  async function load() {
    try {
      users = await api.listUsers();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load accounts.';
    }
  }

  load();

  async function create(event: SubmitEvent) {
    event.preventDefault();
    createError = '';
    creating = true;
    try {
      const created = await api.createUser({
        username: username.trim(),
        password,
        email: null,
        is_admin: isAdmin
      });
      users = [...(users ?? []), created].sort((a, b) =>
        a.username.toLowerCase().localeCompare(b.username.toLowerCase())
      );
      username = '';
      password = '';
      isAdmin = false;
    } catch (e) {
      createError = e instanceof Error ? e.message : 'Could not create that account.';
    } finally {
      creating = false;
    }
  }

  async function submitReset(event: SubmitEvent) {
    event.preventDefault();
    if (!resetTarget) return;
    resetError = '';
    resetDone = '';
    resetting = true;
    try {
      await api.setPassword(resetTarget.id, { new_password: resetPassword });
      resetDone = `Password reset. ${resetTarget.username} has been signed out everywhere.`;
      resetPassword = '';
      resetTarget = null;
    } catch (e) {
      resetError = e instanceof Error ? e.message : 'Could not reset that password.';
    } finally {
      resetting = false;
    }
  }
</script>

<svelte:head><title>Accounts · Grid Reveal</title></svelte:head>

<main class="container">
  <h1>Accounts</h1>
  <p class="muted small lead">
    There is no self-service signup and no password recovery by email. You
    create accounts here, and you reset them here.
  </p>

  <form class="card create" onsubmit={create}>
    <h2>Create an account</h2>
    {#if createError}<p class="error" role="alert">{createError}</p>{/if}

    <div class="pair">
      <div class="field">
        <label for="new-username">Username</label>
        <input
          id="new-username"
          type="text"
          bind:value={username}
          autocapitalize="none"
          autocorrect="off"
          minlength="3"
          maxlength="32"
          required
        />
      </div>
      <div class="field">
        <label for="new-password">Password</label>
        <input id="new-password" type="text" bind:value={password} minlength="8" required />
      </div>
    </div>

    <label class="check">
      <input type="checkbox" bind:checked={isAdmin} />
      <span>Administrator</span>
    </label>

    <button class="btn btn-primary" type="submit" disabled={creating}>
      {creating ? 'Creating…' : 'Create account'}
    </button>
  </form>

  {#if resetDone}<p class="done" role="status">{resetDone}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if !users}
    <p class="muted">Loading…</p>
  {:else}
    <ul class="list">
      {#each users as user (user.id)}
        <li class="card">
          <div class="spread">
            <div>
              <strong>{user.username}</strong>
              {#if user.is_admin}<span class="tag">admin</span>{/if}
            </div>
            <button
              class="btn btn-sm"
              onclick={() => {
                resetTarget = user;
                resetPassword = '';
                resetError = '';
                resetDone = '';
              }}
            >
              Reset password
            </button>
          </div>

          {#if resetTarget?.id === user.id}
            <form class="reset" onsubmit={submitReset}>
              {#if resetError}<p class="error" role="alert">{resetError}</p>{/if}
              <div class="field">
                <label for={`reset-${user.id}`}>New password for {user.username}</label>
                <input
                  id={`reset-${user.id}`}
                  type="text"
                  bind:value={resetPassword}
                  minlength="8"
                  required
                />
              </div>
              <div class="row">
                <button class="btn btn-sm btn-primary" type="submit" disabled={resetting}>
                  {resetting ? 'Resetting…' : 'Set password'}
                </button>
                <button class="btn btn-sm" type="button" onclick={() => (resetTarget = null)}>
                  Cancel
                </button>
              </div>
              <p class="muted small">
                This signs them out of every device. Tell them the new password
                yourself — nothing is emailed.
              </p>
            </form>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  .lead { margin-top: -0.3rem; }
  .create { margin: 1rem 0 1.5rem; }
  .pair { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }

  .check { display: flex; align-items: center; gap: 0.45rem; margin: 0.25rem 0 0.9rem; }
  .check input { width: auto; }

  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.6rem; }

  .tag {
    margin-left: 0.4rem;
    padding: 0.05rem 0.4rem;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 0.72rem;
  }

  .reset { margin-top: 0.85rem; padding-top: 0.85rem; border-top: 1px solid var(--border); }
  .reset p:last-child { margin: 0.5rem 0 0; }

  .done {
    padding: 0.6rem 0.75rem;
    border-radius: 8px;
    background: color-mix(in srgb, var(--success) 14%, transparent);
    color: var(--success);
    border: 1px solid color-mix(in srgb, var(--success) 35%, transparent);
    font-size: 0.9rem;
    margin-bottom: 1rem;
  }

  @media (max-width: 520px) {
    .pair { grid-template-columns: 1fr; }
  }
</style>
