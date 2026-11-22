<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, pictureUrl } from '$lib/api/client';
  import type { GeometryPreview, PictureSummary } from '$lib/api/types';
  import { formatMinutes, today } from '$lib/format';

  /** Days of read-only visibility after a period ends, before it retires. */
  const DEFAULT_AUTO_ARCHIVE_DAYS = 15;

  let title = $state('');
  let pictures = $state<PictureSummary[] | null>(null);
  let selected = $state<PictureSummary | null>(null);

  let targetHours = $state(40);
  let unitMinutes = $state(5);
  /** Null means "let the solver choose"; a number pins the column count. */
  let cols = $state<number | null>(null);

  /**
   * The active period, which every screen has. The start is pre-filled with
   * the administrator's own day — taken from the browser, since the server's
   * day is UTC and would be a day behind for anyone creating a screen late in
   * the evening. The end is the one date they have to decide.
   */
  let startsOn = $state(today());
  let endsOn = $state('');
  /** Blank means the screen never archives itself. */
  let autoArchiveDays = $state<number | null>(DEFAULT_AUTO_ARCHIVE_DAYS);

  let preview = $state<GeometryPreview | null>(null);
  let previewing = $state(false);

  let uploading = $state(false);
  let creating = $state(false);
  let error = $state('');

  const targetMinutes = $derived(Math.max(1, Math.round(targetHours * 60)));
  const windowMissing = $derived(startsOn === '' || endsOn === '');
  const windowInvalid = $derived(!windowMissing && endsOn < startsOn);

  const ready = $derived(
    title.trim() !== '' &&
      selected !== null &&
      preview !== null &&
      !windowMissing &&
      !windowInvalid
  );

  const snapped = $derived(
    preview !== null && preview.exact_total_minutes !== preview.requested_total_minutes
  );

  /**
   * The solver raises the minutes a tile is worth when the target would need
   * more tiles than it allows. Worth saying outright: the number the
   * administrator typed is not the one the screen will use.
   */
  const unitRaised = $derived(
    preview !== null && preview.unit_minutes !== preview.requested_unit_minutes
  );

  async function loadPictures() {
    try {
      pictures = await api.listPictures();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load the picture library.';
    }
  }

  loadPictures();

  async function upload(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    uploading = true;
    error = '';
    try {
      const created = await api.uploadPicture(file);
      pictures = [created, ...(pictures ?? [])];
      selected = created;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not upload that image.';
    } finally {
      uploading = false;
      input.value = '';
    }
  }

  // The preview is recomputed server-side so the solver has exactly one
  // implementation; debounced because it follows every keystroke.
  let debounce: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const picture = selected;
    const minutes = targetMinutes;
    const unit = unitMinutes;
    const fixedCols = cols;

    if (!picture || unit < 1) {
      preview = null;
      return;
    }

    previewing = true;
    clearTimeout(debounce);
    debounce = setTimeout(async () => {
      try {
        preview = await api.geometryPreview({
          picture_id: picture.id,
          target_total_minutes: minutes,
          unit_minutes: unit,
          cols: fixedCols ?? undefined
        });
      } catch (e) {
        error = e instanceof Error ? e.message : 'Could not compute a grid.';
        preview = null;
      } finally {
        previewing = false;
      }
    }, 220);

    return () => clearTimeout(debounce);
  });

  async function create(event: SubmitEvent) {
    event.preventDefault();
    if (!ready || !selected || creating) return;

    creating = true;
    error = '';
    try {
      const screen = await api.createScreen({
        title: title.trim(),
        picture_id: selected.id,
        target_total_minutes: targetMinutes,
        unit_minutes: unitMinutes,
        // null means "let the solver choose the columns".
        cols,
        starts_on: startsOn,
        ends_on: endsOn,
        auto_archive_days: autoArchiveDays
      });
      goto(`/s/${screen.slug}`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not create the screen.';
      creating = false;
    }
  }
</script>

<svelte:head><title>New screen · Grid Reveal</title></svelte:head>

<main class="container">
  <h1>New screen</h1>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <form onsubmit={create}>
    <section class="card">
      <div class="field">
        <label for="title">Title</label>
        <input id="title" type="text" bind:value={title} maxlength="120" required />
      </div>
    </section>

    <section class="card">
      <h2>Background picture</h2>
      <div class="row uploader">
        <label class="btn btn-sm" for="upload">
          {uploading ? 'Uploading…' : 'Upload a new picture'}
        </label>
        <input
          id="upload"
          class="sr-only"
          type="file"
          accept="image/jpeg,image/png,image/webp"
          onchange={upload}
          disabled={uploading}
        />
        <span class="muted small">or choose one below</span>
      </div>

      {#if !pictures}
        <p class="muted small">Loading library…</p>
      {:else if pictures.length === 0}
        <p class="muted small">The library is empty. Upload a picture to begin.</p>
      {:else}
        <ul class="picker">
          {#each pictures as picture (picture.id)}
            <li>
              <button
                type="button"
                class="thumb"
                class:selected={selected?.id === picture.id}
                onclick={() => (selected = picture)}
                aria-pressed={selected?.id === picture.id}
                aria-label={`Use the ${picture.width} by ${picture.height} picture`}
              >
                <img src={pictureUrl(picture.url)} alt="" loading="lazy" />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="card">
      <h2>Target and grid</h2>

      <div class="pair">
        <div class="field">
          <label for="target">Target total (hours)</label>
          <input id="target" type="number" min="1" step="1" bind:value={targetHours} />
        </div>
        <div class="field">
          <label for="unit">Minutes per tile</label>
          <input id="unit" type="number" min="1" step="1" bind:value={unitMinutes} />
        </div>
      </div>

      {#if !selected}
        <p class="muted small">Choose a picture to see the grid.</p>
      {:else if previewing && !preview}
        <p class="muted small">Computing…</p>
      {:else if preview}
        <div class="preview">
          <div class="numbers">
            <div>
              <span class="muted small">Grid</span>
              <strong>{preview.cols} × {preview.rows}</strong>
            </div>
            <div>
              <span class="muted small">Tiles</span>
              <strong>{preview.total_tiles}</strong>
            </div>
            <div>
              <span class="muted small">Minutes per tile</span>
              <strong>{preview.unit_minutes}</strong>
            </div>
            <div>
              <span class="muted small">This screen tracks</span>
              <strong>{formatMinutes(preview.exact_total_minutes)}</strong>
            </div>
          </div>

          {#if unitRaised}
            <!-- The target needed more tiles than the solver allows, so the
                 unit was raised to keep the total. Naming the cause matters:
                 the shape of the picture is not why this one moved. -->
            <p class="muted small snap">
              {preview.requested_unit_minutes} minutes a tile would need more tiles than a
              grid can hold, so tiles are {preview.unit_minutes} minutes each — keeping your
              {formatMinutes(preview.requested_total_minutes)} target at
              {formatMinutes(preview.exact_total_minutes)}.
            </p>
          {:else if snapped}
            <!-- Not every tile count factors into a rectangle matching the
                 picture, so the target is snapped and the real figure shown. -->
            <p class="muted small snap">
              Snapped from your {formatMinutes(preview.requested_total_minutes)} target —
              a grid matching this picture's shape lands on
              {formatMinutes(preview.exact_total_minutes)}.
            </p>
          {/if}

          {#if preview.cell_squareness > 1.15}
            <p class="muted small snap">
              Tiles will be noticeably oblong at this column count.
            </p>
          {/if}

          <div class="field cols">
            <label for="cols">
              Columns
              <span class="muted small">
                {cols === null ? '(chosen to match the picture)' : ''}
              </span>
            </label>
            <div class="row">
              <input
                id="cols"
                type="number"
                min="1"
                max="500"
                value={cols ?? preview.cols}
                oninput={(e) => {
                  const next = Number((e.currentTarget as HTMLInputElement).value);
                  cols = Number.isFinite(next) && next > 0 ? next : null;
                }}
              />
              {#if cols !== null}
                <button class="btn btn-sm" type="button" onclick={() => (cols = null)}>
                  Auto
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/if}
    </section>

    <section class="card">
      <h2>Period</h2>
      <p class="muted small">
        Members can log progress only between these dates. Both are required:
        a screen is a stretch of time with a picture at the end of it.
      </p>

      <div class="pair">
        <div class="field">
          <label for="starts-on">Starts</label>
          <input id="starts-on" type="date" bind:value={startsOn} required />
        </div>
        <div class="field">
          <label for="ends-on">Ends</label>
          <input id="ends-on" type="date" bind:value={endsOn} min={startsOn || undefined} required />
        </div>
      </div>

      {#if windowInvalid}
        <p class="error" role="alert">The end date cannot fall before the start date.</p>
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
        <p class="muted small">
          {#if autoArchiveDays === null}
            The screen stays in the active list, read-only, until you archive it.
          {:else}
            Read-only for {autoArchiveDays}
            {autoArchiveDays === 1 ? 'day' : 'days'} after the period ends, then it
            moves to the archived list on its own.
          {/if}
        </p>
      </div>
    </section>

    <div class="actions">
      <a class="btn" href="/admin">Cancel</a>
      <button class="btn btn-primary" type="submit" disabled={!ready || creating}>
        {creating ? 'Creating…' : 'Create screen'}
      </button>
    </div>
  </form>
</main>

<style>
  form { display: grid; gap: 1rem; }
  .pair { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
  .uploader { margin-bottom: 0.75rem; flex-wrap: wrap; }
  .uploader label { cursor: pointer; }

  .picker {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 0.6rem;
  }

  .thumb {
    width: 100%;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 8px;
    overflow: hidden;
    background: var(--surface-2);
    cursor: pointer;
    display: block;
  }
  .thumb img { width: 100%; aspect-ratio: 4 / 3; object-fit: cover; display: block; }
  .thumb.selected { border-color: var(--accent); }

  .preview { margin-top: 0.5rem; }

  .numbers {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.75rem;
    padding: 0.75rem;
    border-radius: 10px;
    background: var(--surface-2);
  }
  .numbers div { display: grid; gap: 0.1rem; }

  .snap { margin: 0.6rem 0 0; }
  .cols { margin-top: 0.85rem; }
  .cols input { max-width: 120px; }

  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }

  @media (max-width: 520px) {
    .pair { grid-template-columns: 1fr; }
    .numbers { grid-template-columns: 1fr; }
  }
</style>
