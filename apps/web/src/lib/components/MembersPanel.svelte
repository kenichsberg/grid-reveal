<script lang="ts">
  /**
   * Who is on this screen.
   *
   * A plain list, visible to every member. Managing membership stays on the
   * admin page; there is nothing to act on here. What each member has logged
   * remains private — only the screen total is shared.
   */
  import type { MemberSummary } from '$lib/api/types';
  import { auth } from '$lib/stores/auth.svelte';
  import { formatDate } from '$lib/format';

  type Props = {
    members: MemberSummary[] | null;
    /** Shown until the list arrives, so the count never starts blank. */
    participantCount: number;
  };

  let { members, participantCount }: Props = $props();

  const count = $derived(members?.length ?? participantCount);
</script>

<div class="body">
  <header>
    <h2>Members</h2>
    <p class="muted small">{count} {count === 1 ? 'participant' : 'participants'}</p>
  </header>

  {#if !members}
    <p class="muted">Loading…</p>
  {:else if members.length === 0}
    <p class="muted small">No one has been added to this screen yet.</p>
  {:else}
    <ul>
      {#each members as member (member.user_id)}
        <li>
          <span class="name">
            {member.username}
            {#if member.username === auth.user?.username}
              <span class="you">you</span>
            {/if}
          </span>
          <span class="muted small">Joined {formatDate(member.joined_at.slice(0, 10))}</span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .body { padding: 1.1rem 1rem 2rem; }

  header { margin-bottom: 1rem; }
  header h2 { margin: 0; font-size: 1.05rem; }
  header p { margin: 0.15rem 0 0; }

  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 0.4rem; }

  li {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.75rem;
    padding: 0.6rem 0.8rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
  }

  .name {
    display: inline-flex;
    align-items: baseline;
    gap: 0.4rem;
    font-weight: 500;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .you {
    padding: 0.05rem 0.35rem;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 0.72rem;
    font-weight: 400;
  }
</style>
