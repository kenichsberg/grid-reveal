<script lang="ts">
  /**
   * Canvas benchmark. Development harness, not part of the product.
   *
   * Renders a board at a chosen tile count and measures frame times during a
   * reveal animation. Run with `npm run dev` and open /dev/bench.
   *
   * The design assumes a few thousand `fillRect` calls per frame is well
   * within a 2D context's budget and that WebGL is unnecessary. This is how to
   * check that on real hardware before trusting it. If the 95th percentile
   * exceeds ~16.7ms at the sizes you care about, switch BoardCanvas from
   * full-canvas redraws to dirty-rectangle redraws.
   */
  import BoardCanvas from '$lib/components/BoardCanvas.svelte';

  const PRESETS = [
    { label: '600 (25×24)', cols: 25, rows: 24 },
    { label: '2,400 (60×40)', cols: 60, rows: 40 },
    { label: '12,000 (150×80)', cols: 150, rows: 80 },
    { label: '20,000 (200×100)', cols: 200, rows: 100 }
  ];

  let preset = $state(PRESETS[2]);
  let board = $state<ReturnType<typeof BoardCanvas> | null>(null);
  let revealed = $state<number[]>([]);
  let running = $state(false);
  let result = $state<string>('');

  const total = $derived(preset.cols * preset.rows);

  // A 3:2 placeholder so the board has something to letterbox.
  const src =
    'data:image/svg+xml;utf8,' +
    encodeURIComponent(
      `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="800">
         <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
           <stop offset="0" stop-color="#1f6feb"/><stop offset="1" stop-color="#4cc38a"/>
         </linearGradient></defs>
         <rect width="1200" height="800" fill="url(#g)"/>
       </svg>`
    );

  function shuffled(n: number): number[] {
    const tiles = Array.from({ length: n }, (_, i) => i);
    for (let i = tiles.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [tiles[i], tiles[j]] = [tiles[j], tiles[i]];
    }
    return tiles;
  }

  function reset() {
    revealed = [];
    result = '';
  }

  /** Reveals a batch while sampling every frame interval. */
  async function run(batch: number) {
    if (running) return;
    running = true;
    result = '';

    const order = shuffled(total);
    const tiles = order.slice(0, Math.min(batch, total));

    const frames: number[] = [];
    let last = performance.now();
    let sampling = true;

    const sample = () => {
      const now = performance.now();
      frames.push(now - last);
      last = now;
      if (sampling) requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);

    board?.animateReveal(tiles);

    await new Promise((resolve) => setTimeout(resolve, 2200));
    sampling = false;

    // Drop the first frame: it includes the click's own work.
    const measured = frames.slice(1).sort((a, b) => a - b);
    if (measured.length === 0) {
      result = 'No frames captured.';
      running = false;
      return;
    }

    const median = measured[Math.floor(measured.length * 0.5)];
    const p95 = measured[Math.floor(measured.length * 0.95)];
    const worst = measured[measured.length - 1];
    const dropped = measured.filter((f) => f > 16.7).length;

    result =
      `${total.toLocaleString()} tiles · ${tiles.length} revealing · ` +
      `${measured.length} frames\n` +
      `median ${median.toFixed(1)}ms · p95 ${p95.toFixed(1)}ms · worst ${worst.toFixed(1)}ms\n` +
      `${dropped} frame${dropped === 1 ? '' : 's'} over 16.7ms ` +
      `(${((dropped / measured.length) * 100).toFixed(0)}%)`;

    revealed = tiles;
    running = false;
  }
</script>

<svelte:head><title>Canvas benchmark</title></svelte:head>

<div class="stage">
  <BoardCanvas
    bind:this={board}
    {src}
    pictureWidth={1200}
    pictureHeight={800}
    cols={preset.cols}
    rows={preset.rows}
    {revealed}
  />

  <div class="panel">
    <h1>Canvas benchmark</h1>

    <div class="field">
      <label for="preset">Grid</label>
      <select
        id="preset"
        onchange={(e) => {
          const next = PRESETS[Number((e.currentTarget as HTMLSelectElement).value)];
          if (next) preset = next;
          reset();
        }}
      >
        {#each PRESETS as p, i (p.label)}
          <option value={i} selected={p.label === preset.label}>{p.label}</option>
        {/each}
      </select>
    </div>

    <div class="row">
      <button class="btn btn-sm" onclick={() => run(1)} disabled={running}>Reveal 1</button>
      <button class="btn btn-sm" onclick={() => run(12)} disabled={running}>Reveal 12</button>
      <button class="btn btn-sm" onclick={() => run(200)} disabled={running}>Reveal 200</button>
      <button class="btn btn-sm" onclick={reset} disabled={running}>Reset</button>
    </div>

    {#if running}<p class="muted small">Measuring…</p>{/if}
    {#if result}<pre>{result}</pre>{/if}

    <p class="muted small">
      Target: p95 under 16.7ms. Above that, move BoardCanvas to
      dirty-rectangle redraws.
    </p>
  </div>
</div>

<style>
  .stage { position: fixed; inset: 0; background: #000; }

  .panel {
    position: absolute;
    top: 12px;
    left: 12px;
    z-index: 5;
    width: min(340px, calc(100vw - 24px));
    padding: 0.9rem;
    border-radius: 12px;
    background: rgb(0 0 0 / 0.78);
    backdrop-filter: blur(10px);
    border: 1px solid rgb(255 255 255 / 0.16);
    color: #fff;
  }

  h1 { font-size: 1rem; margin: 0 0 0.75rem; }
  .panel :global(.muted) { color: rgb(255 255 255 / 0.62); }
  .panel label { color: rgb(255 255 255 / 0.62); }
  .panel select { background: rgb(255 255 255 / 0.1); border-color: rgb(255 255 255 / 0.2); color: #fff; }
  .panel .btn { background: rgb(255 255 255 / 0.12); border-color: rgb(255 255 255 / 0.22); color: #fff; }

  .row { flex-wrap: wrap; margin-bottom: 0.6rem; }

  pre {
    margin: 0 0 0.6rem;
    padding: 0.6rem;
    border-radius: 8px;
    background: rgb(255 255 255 / 0.08);
    font-size: 0.8rem;
    white-space: pre-wrap;
  }
</style>
