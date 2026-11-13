<script lang="ts">
  import { api, ApiError, pictureUrl } from '$lib/api/client';
  import type { PictureSummary } from '$lib/api/types';

  let pictures = $state<PictureSummary[] | null>(null);
  let error = $state('');
  let notice = $state('');
  let uploading = $state(false);
  let fileInput = $state<HTMLInputElement | null>(null);

  async function load() {
    try {
      pictures = await api.listPictures();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load the library.';
    }
  }

  load();

  async function upload(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    error = '';
    notice = '';
    uploading = true;
    try {
      const created = await api.uploadPicture(file);
      pictures = [created, ...(pictures ?? [])];
      notice = `Uploaded and converted to WebP at ${created.width}×${created.height}.`;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not upload that image.';
    } finally {
      uploading = false;
      if (fileInput) fileInput.value = '';
    }
  }

  async function softDelete(picture: PictureSummary) {
    if (!confirm('Hide this picture from new screens? Screens already using it keep it.')) return;
    error = '';
    notice = '';
    try {
      await api.softDeletePicture(picture.id);
      pictures = (pictures ?? []).filter((p) => p.id !== picture.id);
      notice = 'Hidden from the picker. Existing screens are unaffected.';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not hide that picture.';
    }
  }

  async function hardDelete(picture: PictureSummary) {
    if (!confirm('Permanently delete this picture and its stored file? This cannot be undone.')) {
      return;
    }
    error = '';
    notice = '';
    try {
      await api.hardDeletePicture(picture.id);
      pictures = (pictures ?? []).filter((p) => p.id !== picture.id);
      notice = 'Deleted permanently.';
    } catch (e) {
      // The API refuses while any screen still references the picture, and
      // reports how many do.
      error =
        e instanceof ApiError && e.code === 'picture_in_use'
          ? `${e.message} Hide it instead, or delete those screens first.`
          : e instanceof Error
            ? e.message
            : 'Could not delete that picture.';
    }
  }
</script>

<svelte:head><title>Pictures · Grid Reveal</title></svelte:head>

<main class="container">
  <h1>Picture library</h1>
  <p class="muted small lead">
    Shared across all administrators. Images are downscaled to 2560px and
    converted to WebP on upload; the original is not kept.
  </p>

  <div class="card uploader">
    <label class="btn btn-primary" for="picture-file">
      {uploading ? 'Uploading…' : 'Upload a picture'}
    </label>
    <input
      id="picture-file"
      class="sr-only"
      type="file"
      accept="image/jpeg,image/png,image/webp"
      bind:this={fileInput}
      onchange={upload}
      disabled={uploading}
    />
    <span class="muted small">JPEG, PNG, or WebP · up to 10 MB</span>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  {#if !pictures}
    <p class="muted">Loading…</p>
  {:else if pictures.length === 0}
    <div class="card empty"><p class="muted">No pictures yet.</p></div>
  {:else}
    <ul class="grid">
      {#each pictures as picture (picture.id)}
        <li class="item">
          <img src={pictureUrl(picture.url)} alt="" loading="lazy" />
          <div class="meta">
            <span class="muted small">{picture.width}×{picture.height}</span>
            <div class="row">
              <button class="btn btn-sm" onclick={() => softDelete(picture)}>Hide</button>
              <button class="btn btn-sm btn-danger" onclick={() => hardDelete(picture)}>
                Delete
              </button>
            </div>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  .lead { margin-top: -0.3rem; }

  .uploader {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    flex-wrap: wrap;
    margin: 1rem 0;
  }
  .uploader label { cursor: pointer; }

  .notice {
    padding: 0.55rem 0.7rem;
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 0.88rem;
  }

  .empty { text-align: center; padding: 2rem; }

  .grid {
    list-style: none;
    margin: 1rem 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 0.9rem;
  }

  .item {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--surface);
  }

  .item img {
    width: 100%;
    aspect-ratio: 4 / 3;
    object-fit: cover;
    display: block;
    background: var(--surface-2);
  }

  .meta {
    padding: 0.5rem 0.6rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
</style>
