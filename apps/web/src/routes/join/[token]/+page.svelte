<script lang="ts">
  /**
   * The one page in this app a stranger can open.
   *
   * It shows what is being joined before asking for anything, because the
   * person arriving has only a URL someone sent them and no other way to know
   * what they are agreeing to. Looking costs nothing: the peek endpoint writes
   * nothing, so a reload or a chat client's link preview cannot spend the
   * invite.
   */
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, ApiError, pictureUrl } from '$lib/api/client';
  import { auth } from '$lib/stores/auth.svelte';
  import type { InvitePeek } from '$lib/api/types';

  const token = $derived(page.params.token!);

  let invite = $state<InvitePeek | null>(null);
  let loadError = $state('');
  let username = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);

  $effect(() => {
    void load(token);
  });

  async function load(t: string) {
    invite = null;
    loadError = '';
    try {
      invite = await api.peekInvite(t);
    } catch (e) {
      loadError =
        e instanceof ApiError && e.status === 404
          ? 'This invite link is not valid.'
          : 'Could not load this invite.';
    }
  }

  async function join(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    error = '';
    busy = true;
    try {
      // Credentials are omitted when a session is already present: the invite
      // then joins that account rather than creating a second one.
      const body = invite?.viewer_is_signed_in
        ? { username: null, password: null }
        : { username, password };

      const joined = await api.acceptInvite(token, body);
      auth.adopt(joined.user);
      goto(`/s/${joined.slug}`);
    } catch (e) {
      error = e instanceof ApiError ? e.message : 'Could not accept this invite.';
      // The invite may have been spent or withdrawn while this page was open,
      // so re-read its state rather than leaving a form that cannot succeed.
      await load(token);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>Join · Grid Reveal</title></svelte:head>

<main class="wrap">
  <div class="card">
    <h1>Grid Reveal</h1>

    {#if loadError}
      <p class="error" role="alert">{loadError}</p>
    {:else if !invite}
      <p class="muted small">Loading…</p>
    {:else}
      <img class="preview" src={pictureUrl(invite.picture_url)} alt="" />
      <p class="muted small">You have been invited to</p>
      <h2>{invite.screen_title}</h2>

      {#if !invite.open}
        <p class="error" role="alert">{invite.closed_reason}</p>
        <p class="muted small note">
          Ask whoever sent you this link for a new one.
        </p>
      {:else}
        {#if error}<p class="error" role="alert">{error}</p>{/if}

        <form onsubmit={join}>
          {#if invite.viewer_is_signed_in}
            <p class="muted small">
              You are already signed in. Joining adds this screen to the account
              you are using.
            </p>
          {:else}
            <div class="field">
              <label for="username">Choose a username</label>
              <input
                id="username"
                type="text"
                bind:value={username}
                autocomplete="username"
                autocapitalize="none"
                autocorrect="off"
                minlength="3"
                maxlength="32"
                required
              />
            </div>

            <div class="field">
              <label for="password">Choose a password</label>
              <input
                id="password"
                type="password"
                bind:value={password}
                autocomplete="new-password"
                minlength="8"
                required
              />
              <p class="muted small">
                Only you will know it. Nobody can see it, including the person
                who invited you.
              </p>
            </div>
          {/if}

          <button class="btn btn-primary full" type="submit" disabled={busy}>
            {busy ? 'Joining…' : invite.viewer_is_signed_in ? 'Join this screen' : 'Create account and join'}
          </button>
        </form>
      {/if}
    {/if}
  </div>
</main>

<style>
  .wrap { min-height: 100dvh; display: grid; place-items: center; padding: 1.5rem 16px; }
  .card { width: 100%; max-width: 380px; }
  h2 { margin: 0.1rem 0 1rem; font-size: 1.25rem; }
  .preview {
    width: 100%;
    aspect-ratio: 4 / 3;
    object-fit: cover;
    border-radius: 8px;
    margin-bottom: 0.9rem;
  }
  .full { width: 100%; }
  .note { margin: 0.9rem 0 0; }
</style>
