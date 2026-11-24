<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, inviteUrl, pictureUrl } from '$lib/api/client';
  import { pageTitle } from '$lib/stores/site.svelte';
  import type {
    MemberSummary,
    InviteSummary,
    PictureSummary,
    RemovalImpact,
    ScreenDetail,
    UserSummary
  } from '$lib/api/types';
  import { formatDayMonth, formatInstantDay, formatMinutes, today } from '$lib/format';

  const slug = $derived(page.params.slug!);

  let screen = $state<ScreenDetail | null>(null);
  let members = $state<MemberSummary[]>([]);
  let users = $state<UserSummary[]>([]);
  let pictures = $state<PictureSummary[]>([]);
  let error = $state('');
  let notice = $state('');

  // Title
  let title = $state('');
  let savingTitle = $state(false);

  // Wording. Empty means "use the app's default", which is also what the API
  // stores for a cleared label — so the inputs need no third, "unset" state.
  let progressTitle = $state('');
  let entryNoun = $state('');
  let savingWording = $state(false);

  // Geometry
  let cols = $state(0);
  let rows = $state(0);
  let unitMinutes = $state(0);
  let savingGeometry = $state(false);

  // Period
  let startsOn = $state('');
  let endsOn = $state('');
  let autoArchiveDays = $state<number | null>(null);
  let savingPeriod = $state(false);

  // Membership
  let addUserId = $state('');
  let removing = $state<MemberSummary | null>(null);
  let impact = $state<RemovalImpact | null>(null);
  let deleteTheirEntries = $state(false);

  // Clone
  let invites = $state<InviteSummary[]>([]);
  let inviteLabel = $state('');
  let generating = $state(false);
  /** The token just generated, so its link is shown once without hunting. */
  let freshToken = $state('');
  let copied = $state('');

  let cloneTitle = $state('');
  let clonePictureId = $state('');
  /** A clone is a new period rather than a copy, and every screen needs one. */
  let cloneStartsOn = $state(today());
  let cloneEndsOn = $state('');
  let cloning = $state(false);

  // Delete
  let confirmTitle = $state('');
  let deleting = $state(false);

  const nonMembers = $derived(
    users.filter((user) => !members.some((member) => member.user_id === user.id))
  );

  /** Mirrors the server's derivation so the admin sees the consequence first. */
  const geometryOutcome = $derived.by(() => {
    if (!screen || cols < 1 || rows < 1 || unitMinutes < 1) return null;
    const tiles = cols * rows;
    const revealed = Math.min(Math.floor(screen.total_minutes / unitMinutes), tiles);
    return {
      tiles,
      revealed,
      regresses: revealed < screen.revealed_count,
      changed:
        cols !== screen.cols || rows !== screen.rows || unitMinutes !== screen.unit_minutes
    };
  });

  async function load() {
    error = '';
    try {
      const detail = await api.getScreen(slug);
      screen = detail;
      title = detail.title;
      progressTitle = detail.progress_title ?? '';
      entryNoun = detail.entry_noun ?? '';
      cols = detail.cols;
      rows = detail.rows;
      unitMinutes = detail.unit_minutes;
      startsOn = detail.starts_on;
      endsOn = detail.ends_on;
      autoArchiveDays = detail.auto_archive_days;
      cloneTitle = '';
      clonePictureId = '';
      cloneStartsOn = today();
      cloneEndsOn = '';

      const [memberList, userList, pictureList, inviteList] = await Promise.all([
        api.listMembers(slug),
        api.listUsers(),
        api.listPictures(),
        api.listInvites(slug)
      ]);
      members = memberList;
      users = userList;
      pictures = pictureList;
      invites = inviteList;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load this screen.';
    }
  }

  $effect(() => {
    void slug;
    load();
  });

  function report(message: string) {
    notice = message;
    error = '';
  }

  function fail(e: unknown, fallback: string) {
    error = e instanceof Error ? e.message : fallback;
    notice = '';
  }

  const openInvites = $derived(invites.filter((invite) => invite.open));
  const spentInvites = $derived(invites.filter((invite) => !invite.open));

  async function generateInvite(event: SubmitEvent) {
    event.preventDefault();
    if (generating) return;
    generating = true;
    try {
      const created = await api.createInvite(slug, { label: inviteLabel.trim() || null });
      freshToken = created.token;
      inviteLabel = '';
      invites = await api.listInvites(slug);
      report('Invite link created.');
    } catch (e) {
      fail(e, 'Could not create an invite.');
    } finally {
      generating = false;
    }
  }

  async function copyInvite(token: string) {
    try {
      await navigator.clipboard.writeText(inviteUrl(token));
      copied = token;
    } catch {
      // Clipboard access can be refused; the link is on screen either way.
      copied = '';
      fail(new Error('Could not copy. Select the link and copy it by hand.'), '');
    }
  }

  async function revokeInvite(invite: InviteSummary) {
    try {
      await api.revokeInvite(slug, invite.id);
      if (freshToken === invite.token) freshToken = '';
      invites = await api.listInvites(slug);
      report('Invite withdrawn.');
    } catch (e) {
      fail(e, 'Could not withdraw that invite.');
    }
  }

  async function saveTitle(event: SubmitEvent) {
    event.preventDefault();
    savingTitle = true;
    try {
      screen = await api.updateScreen(slug, {
        title: title.trim(),
        cols: null,
        rows: null,
        unit_minutes: null,
        // Absent leaves the period untouched.
        window: null,
        progress_title: null,
        entry_noun: null
      });
      report('Title saved.');
    } catch (e) {
      fail(e, 'Could not save the title.');
    } finally {
      savingTitle = false;
    }
  }

  async function saveWording(event: SubmitEvent) {
    event.preventDefault();
    savingWording = true;
    try {
      // An emptied field is sent as '' rather than null: null would mean
      // "leave it as it is", and there would be no way back to the default.
      screen = await api.updateScreen(slug, {
        title: null,
        cols: null,
        rows: null,
        unit_minutes: null,
        window: null,
        progress_title: progressTitle.trim(),
        entry_noun: entryNoun.trim()
      });
      progressTitle = screen.progress_title ?? '';
      entryNoun = screen.entry_noun ?? '';
      report('Wording saved.');
    } catch (e) {
      fail(e, 'Could not save the wording.');
    } finally {
      savingWording = false;
    }
  }

  async function saveGeometry() {
    if (!geometryOutcome?.changed) return;
    const warning = geometryOutcome.regresses
      ? `This re-covers part of the picture: ${screen?.revealed_count} tiles revealed becomes ${geometryOutcome.revealed}. The reveal order also changes. No logged time is lost. Continue?`
      : `The grid becomes ${cols} × ${rows} (${geometryOutcome.tiles} tiles) and the reveal order changes. No logged time is lost. Continue?`;
    if (!confirm(warning)) return;

    savingGeometry = true;
    try {
      screen = await api.updateScreen(slug, {
        title: null,
        cols,
        rows,
        unit_minutes: unitMinutes,
        window: null,
        progress_title: null,
        entry_noun: null
      });
      report('Grid updated. Total logged time is unchanged.');
    } catch (e) {
      fail(e, 'Could not update the grid.');
    } finally {
      savingGeometry = false;
    }
  }

  async function addMember() {
    if (!addUserId) return;
    try {
      await api.addMember(slug, { user_id: addUserId });
      members = await api.listMembers(slug);
      addUserId = '';
      report('Member added.');
    } catch (e) {
      fail(e, 'Could not add that member.');
    }
  }

  async function startRemoval(member: MemberSummary) {
    removing = member;
    impact = null;
    deleteTheirEntries = false;
    try {
      impact = await api.removalImpact(slug, member.user_id);
    } catch (e) {
      fail(e, 'Could not check what removing them would change.');
    }
  }

  async function confirmRemoval() {
    if (!removing) return;
    try {
      await api.removeMember(slug, removing.user_id, { delete_entries: deleteTheirEntries });
      members = await api.listMembers(slug);
      screen = await api.getScreen(slug);
      report(
        deleteTheirEntries
          ? 'Member removed and their entries deleted. The board has rewound.'
          : 'Member removed. Their logged time stays in the total.'
      );
      removing = null;
      impact = null;
    } catch (e) {
      fail(e, 'Could not remove that member.');
    }
  }

  const cloneReady = $derived(
    cloneTitle.trim() !== '' &&
      clonePictureId !== '' &&
      cloneStartsOn !== '' &&
      cloneEndsOn !== '' &&
      cloneEndsOn >= cloneStartsOn
  );

  const periodMissing = $derived(startsOn === '' || endsOn === '');
  const periodInvalid = $derived(!periodMissing && endsOn < startsOn);

  const periodChanged = $derived(
    screen !== null &&
      (startsOn !== screen.starts_on ||
        endsOn !== screen.ends_on ||
        autoArchiveDays !== screen.auto_archive_days)
  );

  async function savePeriod(event: SubmitEvent) {
    event.preventDefault();
    if (periodMissing || periodInvalid || savingPeriod) return;
    savingPeriod = true;
    try {
      // Sent whole: the window replaces the stored period. Omitting it
      // entirely is what leaves the period alone, which is how the other forms
      // on this page save without restating the dates.
      screen = await api.updateScreen(slug, {
        title: null,
        cols: null,
        rows: null,
        unit_minutes: null,
        window: {
          starts_on: startsOn,
          ends_on: endsOn,
          auto_archive_days: autoArchiveDays
        },
        progress_title: null,
        entry_noun: null
      });
      report('Period saved.');
    } catch (e) {
      fail(e, 'Could not save the period.');
    } finally {
      savingPeriod = false;
    }
  }

  async function clone(event: SubmitEvent) {
    event.preventDefault();
    if (!cloneTitle.trim() || !clonePictureId || !cloneReady) return;
    cloning = true;
    try {
      const created = await api.cloneScreen(slug, {
        title: cloneTitle.trim(),
        picture_id: clonePictureId,
        // A clone is a new period rather than a copy of the source's.
        starts_on: cloneStartsOn,
        ends_on: cloneEndsOn,
        auto_archive_days: null
      });
      goto(`/admin/screens/${created.slug}`);
    } catch (e) {
      fail(e, 'Could not clone this screen.');
      cloning = false;
    }
  }

  async function toggleArchive() {
    if (!screen) return;
    try {
      if (screen.archived) {
        await api.unarchiveScreen(slug);
        report('Unarchived. Members can log progress again.');
      } else {
        await api.archiveScreen(slug);
        report('Archived. The screen is now read-only.');
      }
      screen = await api.getScreen(slug);
    } catch (e) {
      fail(e, 'Could not change the archive state.');
    }
  }

  async function destroy(event: SubmitEvent) {
    event.preventDefault();
    if (!screen || confirmTitle !== screen.title) return;
    deleting = true;
    try {
      await api.deleteScreen(slug, { confirm_title: confirmTitle });
      goto('/');
    } catch (e) {
      fail(e, 'Could not delete this screen.');
      deleting = false;
    }
  }
</script>

<svelte:head><title>{pageTitle(`Manage ${screen?.title ?? 'screen'}`)}</title></svelte:head>

<main class="container">
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  {#if !screen}
    <p class="muted">Loading…</p>
  {:else}
    <div class="spread head">
      <h1>Manage screen</h1>
      <a class="btn btn-sm" href={`/s/${slug}`}>Open board</a>
    </div>

    <section class="card">
      <form onsubmit={saveTitle}>
        <div class="field">
          <label for="title">Title</label>
          <input id="title" type="text" bind:value={title} maxlength="120" required />
        </div>
        <button class="btn btn-sm btn-primary" type="submit" disabled={savingTitle}>
          {savingTitle ? 'Saving…' : 'Save title'}
        </button>
      </form>
    </section>

    <section class="card">
      <h2>Wording</h2>
      <p class="muted small">
        What this board calls its progress and the sessions logged against it.
        Leave a field empty to use the default.
      </p>

      <form onsubmit={saveWording}>
        <div class="field">
          <label for="progress-title">Progress heading</label>
          <input
            id="progress-title"
            type="text"
            bind:value={progressTitle}
            maxlength="60"
            placeholder="Progress"
          />
        </div>

        <div class="field">
          <label for="entry-noun">Name for one entry</label>
          <input
            id="entry-noun"
            type="text"
            bind:value={entryNoun}
            maxlength="40"
            placeholder="entry"
          />
          <p class="muted small">
            Used as written, so “Daimoku” gives “Add Daimoku” and “Edit Daimoku”.
          </p>
        </div>

        <button class="btn btn-sm btn-primary" type="submit" disabled={savingWording}>
          {savingWording ? 'Saving…' : 'Save wording'}
        </button>
      </form>
    </section>

    <section class="card">
      <h2>Grid</h2>
      <p class="muted small">
        {formatMinutes(screen.total_minutes)} logged ·
        {screen.revealed_count}/{screen.total_tiles} tiles revealed
      </p>

      <div class="triple">
        <div class="field">
          <label for="cols">Columns</label>
          <input id="cols" type="number" min="1" max="500" bind:value={cols} />
        </div>
        <div class="field">
          <label for="rows">Rows</label>
          <input id="rows" type="number" min="1" max="500" bind:value={rows} />
        </div>
        <div class="field">
          <label for="unit">Minutes per tile</label>
          <input id="unit" type="number" min="1" bind:value={unitMinutes} />
        </div>
      </div>

      {#if geometryOutcome?.changed}
        <div class="warn" class:regress={geometryOutcome.regresses}>
          <p>
            <strong>{geometryOutcome.tiles} tiles</strong>, of which
            <strong>{geometryOutcome.revealed}</strong> would be revealed.
          </p>
          {#if geometryOutcome.regresses}
            <p class="small">
              The picture becomes less revealed than it is now
              ({screen.revealed_count} → {geometryOutcome.revealed}). The reveal
              order is reshuffled. No logged time is lost.
            </p>
          {:else}
            <p class="small">
              The reveal order is reshuffled. No logged time is lost.
            </p>
          {/if}
        </div>
      {/if}

      <button
        class="btn btn-sm btn-primary"
        onclick={saveGeometry}
        disabled={!geometryOutcome?.changed || savingGeometry}
      >
        {savingGeometry ? 'Saving…' : 'Update grid'}
      </button>
    </section>

    <section class="card">
      <h2>Members</h2>

      {#if members.length === 0}
        <p class="muted small">No members yet.</p>
      {:else}
        <ul class="members">
          {#each members as member (member.user_id)}
            <li>
              <span>{member.username}</span>
              <button class="btn btn-sm btn-danger" onclick={() => startRemoval(member)}>
                Remove
              </button>
            </li>
          {/each}
        </ul>
      {/if}

      {#if removing}
        <div class="removal">
          <h3>Remove {removing.username}?</h3>
          {#if !impact}
            <p class="muted small">Checking what this would change…</p>
          {:else}
            <p class="small">
              They have logged <strong>{formatMinutes(impact.minutes)}</strong>
              across {impact.entry_count}
              {impact.entry_count === 1 ? 'entry' : 'entries'}.
            </p>
            <label class="check">
              <input type="checkbox" bind:checked={deleteTheirEntries} />
              <span>
                Also delete their entries
                {#if impact.revealed_tiles_lost > 0}
                  <span class="muted small">
                    — the board rewinds by {impact.revealed_tiles_lost}
                    {impact.revealed_tiles_lost === 1 ? 'tile' : 'tiles'}
                  </span>
                {/if}
              </span>
            </label>
            <p class="muted small">
              {deleteTheirEntries
                ? 'Their logged time is removed from the screen total.'
                : 'Their logged time stays in the screen total.'}
            </p>
          {/if}
          <div class="row">
            <button class="btn btn-sm btn-danger" onclick={confirmRemoval} disabled={!impact}>
              Remove
            </button>
            <button class="btn btn-sm" onclick={() => (removing = null)}>Cancel</button>
          </div>
        </div>
      {/if}

      {#if nonMembers.length > 0}
        <div class="add row">
          <select bind:value={addUserId} aria-label="Account to add">
            <option value="">Add an account…</option>
            {#each nonMembers as user (user.id)}
              <option value={user.id}>{user.username}</option>
            {/each}
          </select>
          <button class="btn btn-sm" onclick={addMember} disabled={!addUserId}>Add</button>
        </div>
      {/if}
    </section>

    <section class="card">
      <h2>Invite links</h2>
      <p class="muted small">
        A link joins one person to this screen. They choose their own username
        and password, so you never handle either. Each link works once, and
        every link for this screen stops working after
        {formatDayMonth(screen.ends_on)} — move the end date to extend them.
      </p>

      <form class="row" onsubmit={generateInvite}>
        <input
          type="text"
          bind:value={inviteLabel}
          maxlength="80"
          placeholder="Who is it for? (optional)"
          aria-label="Who the invite is for"
        />
        <button class="btn btn-sm btn-primary" type="submit" disabled={generating}>
          {generating ? 'Creating…' : 'New link'}
        </button>
      </form>

      {#if freshToken}
        <div class="fresh">
          <code>{inviteUrl(freshToken)}</code>
          <button class="btn btn-sm" onclick={() => copyInvite(freshToken)}>
            {copied === freshToken ? 'Copied' : 'Copy'}
          </button>
        </div>
      {/if}

      {#if openInvites.length === 0}
        <p class="muted small">No links are waiting to be used.</p>
      {:else}
        <ul class="invites">
          {#each openInvites as invite (invite.id)}
            <li>
              <span>
                {invite.label ?? 'Unlabelled'}
                <span class="muted small">· {formatInstantDay(invite.created_at)}</span>
              </span>
              <span class="row">
                <button class="btn btn-sm" onclick={() => copyInvite(invite.token)}>
                  {copied === invite.token ? 'Copied' : 'Copy link'}
                </button>
                <button class="btn btn-sm btn-danger" onclick={() => revokeInvite(invite)}>
                  Withdraw
                </button>
              </span>
            </li>
          {/each}
        </ul>
      {/if}

      {#if spentInvites.length > 0}
        <h3 class="small">Already used or closed</h3>
        <ul class="invites muted small">
          {#each spentInvites as invite (invite.id)}
            <li>
              <span>{invite.label ?? 'Unlabelled'}</span>
              <span>
                {#if invite.accepted_by}
                  joined as {invite.accepted_by}
                {:else if invite.accepted_at}
                  used, by an account since deleted
                {:else if invite.revoked_at}
                  withdrawn
                {:else}
                  expired with the period
                {/if}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="card">
      <h2>Clone</h2>
      <p class="muted small">
        Copies the grid, members, and goals into a fresh board with nothing
        revealed. A new title, a picture, and a period of its own are required.
      </p>
      <form onsubmit={clone}>
        <div class="field">
          <label for="clone-title">New title</label>
          <input id="clone-title" type="text" bind:value={cloneTitle} maxlength="120" required />
        </div>
        <div class="field">
          <label for="clone-picture">Picture</label>
          <select id="clone-picture" bind:value={clonePictureId} required>
            <option value="">Choose a picture…</option>
            {#each pictures as picture (picture.id)}
              <option value={picture.id}>{picture.width}×{picture.height}</option>
            {/each}
          </select>
        </div>
        {#if clonePictureId}
          {@const chosen = pictures.find((p) => p.id === clonePictureId)}
          {#if chosen}
            <img class="clone-preview" src={pictureUrl(chosen.url)} alt="" />
          {/if}
        {/if}
        <div class="pair">
          <div class="field">
            <label for="clone-starts-on">Starts</label>
            <input id="clone-starts-on" type="date" bind:value={cloneStartsOn} required />
          </div>
          <div class="field">
            <label for="clone-ends-on">Ends</label>
            <input
              id="clone-ends-on"
              type="date"
              bind:value={cloneEndsOn}
              min={cloneStartsOn || undefined}
              required
            />
          </div>
        </div>
        <button class="btn btn-sm btn-primary" type="submit" disabled={cloning || !cloneReady}>
          {cloning ? 'Cloning…' : 'Clone screen'}
        </button>
      </form>
    </section>

    <section class="card">
      <h2>Period</h2>
      <p class="muted small">
        Progress can be logged only between these dates, and only for dates
        inside them. Once the period ends the screen becomes read-only, keeping
        its board and celebration. Both dates are required; to reopen a closed
        screen, move its end date.
      </p>

      <form class="stack" onsubmit={savePeriod}>
        <div class="pair">
          <div class="field">
            <label for="starts-on">Starts</label>
            <input id="starts-on" type="date" bind:value={startsOn} required />
          </div>
          <div class="field">
            <label for="ends-on">Ends</label>
            <input
              id="ends-on"
              type="date"
              bind:value={endsOn}
              min={startsOn || undefined}
              required
            />
          </div>
        </div>

        {#if periodInvalid}
          <p class="error" role="alert">The end date cannot fall before the start date.</p>
        {:else if periodMissing}
          <p class="error" role="alert">A screen needs both a start date and an end date.</p>
        {/if}

        <div class="field">
          <label for="auto-archive">Archive automatically after</label>
          <div class="row">
            <input
              id="auto-archive"
              type="number"
              min="0"
              placeholder="never"
              value={autoArchiveDays ?? ''}
              oninput={(e) => {
                const raw = (e.currentTarget as HTMLInputElement).value;
                const next = Number(raw);
                autoArchiveDays = raw !== '' && Number.isFinite(next) && next >= 0 ? next : null;
              }}
            />
            <span class="muted small">days past the end date</span>
          </div>
        </div>

        <div class="row">
          <button
            class="btn btn-sm btn-primary"
            type="submit"
            disabled={!periodChanged || periodMissing || periodInvalid || savingPeriod}
          >
            {savingPeriod ? 'Saving…' : 'Save period'}
          </button>
          {#if screen.phase !== 'active'}
            <span class="muted small">
              {screen.phase === 'upcoming'
                ? 'Not started yet.'
                : screen.phase === 'ended'
                  ? 'Ended — extend the end date to reopen it.'
                  : 'Archived.'}
            </span>
          {/if}
        </div>
      </form>
    </section>

    <section class="card">
      <h2>{screen.archived ? 'Archived' : 'Archive'}</h2>
      <p class="muted small">
        {screen.archived
          ? 'This screen is read-only. Members can still view it.'
          : 'Archiving makes the screen read-only. Members keep access.'}
        {#if screen.auto_archive_days !== null}
          It also archives itself {screen.auto_archive_days}
          {screen.auto_archive_days === 1 ? 'day' : 'days'} after the period ends.
        {/if}
      </p>
      <button class="btn btn-sm" onclick={toggleArchive}>
        {screen.archived ? 'Unarchive' : 'Archive'}
      </button>
      {#if screen.archived && screen.auto_archive_days !== null}
        <p class="muted small">
          Unarchiving also cancels the automatic archive, so it will not retire
          itself again.
        </p>
      {/if}
    </section>

    <section class="card danger">
      <h2>Delete</h2>
      <p class="small">
        Permanently deletes this screen with its entries, goals, and membership.
        The picture stays in the library. This cannot be undone.
      </p>
      <form onsubmit={destroy}>
        <div class="field">
          <label for="confirm">Type <strong>{screen.title}</strong> to confirm</label>
          <input id="confirm" type="text" bind:value={confirmTitle} autocomplete="off" />
        </div>
        <button
          class="btn btn-sm btn-danger"
          type="submit"
          disabled={confirmTitle !== screen.title || deleting}
        >
          {deleting ? 'Deleting…' : 'Delete this screen'}
        </button>
      </form>
    </section>
  {/if}
</main>

<style>
  main { display: grid; gap: 1rem; }
  .head { margin-bottom: 0; }

  section h2 { margin-top: 0; }

  .triple { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem; }

  .warn {
    margin: 0.5rem 0 0.9rem;
    padding: 0.7rem 0.8rem;
    border-radius: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .warn p { margin: 0 0 0.25rem; }
  .warn p:last-child { margin: 0; }
  .warn.regress {
    background: var(--danger-bg);
    border-color: color-mix(in srgb, var(--danger) 35%, transparent);
  }

  .members { list-style: none; margin: 0 0 0.85rem; padding: 0; display: grid; gap: 0.4rem; }
  .members li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.45rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .removal {
    margin: 0.85rem 0;
    padding: 0.85rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
  }
  .removal h3 { margin-top: 0; }

  .check { display: flex; align-items: flex-start; gap: 0.45rem; margin: 0.5rem 0; }
  .check input { width: auto; margin-top: 0.3rem; }

  .add { margin-top: 0.5rem; }

  .invites { list-style: none; margin: 0.6rem 0 0; padding: 0; display: grid; gap: 0.4rem; }
  .invites li { display: flex; align-items: center; justify-content: space-between; gap: 0.6rem; }
  .fresh {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0.6rem 0;
    padding: 0.5rem 0.6rem;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.04);
  }
  .fresh code { overflow-wrap: anywhere; font-size: 0.85rem; }

  .clone-preview {
    display: block;
    width: 100%;
    max-width: 240px;
    aspect-ratio: 4 / 3;
    object-fit: cover;
    border-radius: 8px;
    margin-bottom: 0.85rem;
  }

  .danger { border-color: color-mix(in srgb, var(--danger) 40%, var(--border)); }

  .notice {
    padding: 0.55rem 0.7rem;
    border-radius: 8px;
    background: color-mix(in srgb, var(--success) 14%, transparent);
    color: var(--success);
    border: 1px solid color-mix(in srgb, var(--success) 35%, transparent);
    font-size: 0.88rem;
  }

  @media (max-width: 520px) {
    .triple { grid-template-columns: 1fr; }
  }
</style>
