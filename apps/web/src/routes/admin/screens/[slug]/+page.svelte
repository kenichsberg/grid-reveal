<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, pictureUrl } from '$lib/api/client';
  import type {
    MemberSummary,
    PictureSummary,
    RemovalImpact,
    ScreenDetail,
    UserSummary
  } from '$lib/api/types';
  import { formatMinutes } from '$lib/format';

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
  let cloneTitle = $state('');
  let clonePictureId = $state('');
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
      cols = detail.cols;
      rows = detail.rows;
      unitMinutes = detail.unit_minutes;
      startsOn = detail.starts_on ?? '';
      endsOn = detail.ends_on ?? '';
      autoArchiveDays = detail.auto_archive_days;
      cloneTitle = '';
      clonePictureId = detail.picture_url ? '' : '';

      const [memberList, userList, pictureList] = await Promise.all([
        api.listMembers(slug),
        api.listUsers(),
        api.listPictures()
      ]);
      members = memberList;
      users = userList;
      pictures = pictureList;
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
        window: null
      });
      report('Title saved.');
    } catch (e) {
      fail(e, 'Could not save the title.');
    } finally {
      savingTitle = false;
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
        window: null
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

  const periodInvalid = $derived(startsOn !== '' && endsOn !== '' && endsOn < startsOn);

  const periodChanged = $derived(
    screen !== null &&
      ((startsOn || null) !== screen.starts_on ||
        (endsOn || null) !== screen.ends_on ||
        (endsOn ? autoArchiveDays : null) !== screen.auto_archive_days)
  );

  async function savePeriod(event: SubmitEvent) {
    event.preventDefault();
    if (periodInvalid || savingPeriod) return;
    savingPeriod = true;
    try {
      // Sent whole: a null bound inside the window clears that bound, whereas
      // omitting the window entirely would leave the period alone.
      screen = await api.updateScreen(slug, {
        title: null,
        cols: null,
        rows: null,
        unit_minutes: null,
        window: {
          starts_on: startsOn || null,
          ends_on: endsOn || null,
          auto_archive_days: endsOn ? autoArchiveDays : null
        }
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
    if (!cloneTitle.trim() || !clonePictureId) return;
    cloning = true;
    try {
      const created = await api.cloneScreen(slug, {
        title: cloneTitle.trim(),
        picture_id: clonePictureId,
        // A clone is a new period, so it starts without one until set.
        starts_on: null,
        ends_on: null,
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

<svelte:head><title>Manage {screen?.title ?? 'screen'} · Grid Reveal</title></svelte:head>

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
      <h2>Clone</h2>
      <p class="muted small">
        Copies the grid, members, and goals into a fresh board with nothing
        revealed. A new title and picture are required.
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
        <button
          class="btn btn-sm btn-primary"
          type="submit"
          disabled={cloning || !cloneTitle.trim() || !clonePictureId}
        >
          {cloning ? 'Cloning…' : 'Clone screen'}
        </button>
      </form>
    </section>

    <section class="card">
      <h2>Period</h2>
      <p class="muted small">
        Progress can be logged only between these dates, and only for dates
        inside them. Once the period ends the screen becomes read-only, keeping
        its board and celebration. Leave both blank to keep it open
        indefinitely.
      </p>

      <form class="stack" onsubmit={savePeriod}>
        <div class="pair">
          <div class="field">
            <label for="starts-on">Starts</label>
            <input id="starts-on" type="date" bind:value={startsOn} />
          </div>
          <div class="field">
            <label for="ends-on">Ends</label>
            <input id="ends-on" type="date" bind:value={endsOn} min={startsOn || undefined} />
          </div>
        </div>

        {#if periodInvalid}
          <p class="error" role="alert">The end date cannot fall before the start date.</p>
        {/if}

        {#if endsOn}
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
        {/if}

        <div class="row">
          <button
            class="btn btn-sm btn-primary"
            type="submit"
            disabled={!periodChanged || periodInvalid || savingPeriod}
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
