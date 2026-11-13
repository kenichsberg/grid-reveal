<script lang="ts">
  /**
   * The board: a picture letterboxed into the viewport with a canvas of cover
   * tiles laid exactly over it.
   *
   * Canvas rather than DOM nodes because grids run to thousands of tiles. 2D
   * rather than WebGL because a few thousand rectangles per frame is well
   * within a 2D context's budget — and the canvas only redraws while an
   * animation is running, staying completely idle the rest of the time.
   *
   * The static mass of covered tiles is drawn as exactly two calls (one filled
   * Path2D, one stroked Path2D) regardless of tile count; only the handful of
   * tiles currently animating are drawn individually.
   */
  import { onMount } from 'svelte';

  type Props = {
    src: string;
    pictureWidth: number;
    pictureHeight: number;
    cols: number;
    rows: number;
    /** Every revealed tile index. Changes to this snap in without animation. */
    revealed: number[];
    /** Fired once a reveal animation finishes. */
    onanimationend?: () => void;
  };

  let {
    src,
    pictureWidth,
    pictureHeight,
    cols,
    rows,
    revealed,
    onanimationend
  }: Props = $props();

  const TILE_DURATION = 420;
  const STAGGER = 26;
  /** Keeps a big simultaneous reveal from dragging on; tiles overlap instead. */
  const MAX_TOTAL_STAGGER = 1400;

  let container = $state<HTMLDivElement | null>(null);
  let canvas = $state<HTMLCanvasElement | null>(null);

  let containerWidth = $state(0);
  let containerHeight = $state(0);

  /** Tiles revealed and fully drawn as absent. */
  let settled = new Set<number>();
  /** Tile index -> animation start timestamp. */
  let animating = new Map<number, number>();
  let frame = 0;

  let tileColor = '#8b8f96';
  let lineColor = '#74787f';

  const aspect = $derived(
    pictureHeight > 0 ? pictureWidth / pictureHeight : 1
  );

  /** The letterboxed picture box: the grid overlays exactly this rectangle. */
  const box = $derived.by(() => {
    if (containerWidth <= 0 || containerHeight <= 0) {
      return { left: 0, top: 0, width: 0, height: 0 };
    }
    // Fit entirely inside the viewport, letterboxing whichever axis is slack.
    let width: number;
    let height: number;
    if (containerWidth / containerHeight > aspect) {
      height = containerHeight;
      width = height * aspect;
    } else {
      width = containerWidth;
      height = width / aspect;
    }
    return {
      left: (containerWidth - width) / 2,
      top: (containerHeight - height) / 2,
      width,
      height
    };
  });

  function prefersReducedMotion() {
    return (
      typeof matchMedia === 'function' &&
      matchMedia('(prefers-reduced-motion: reduce)').matches
    );
  }

  function readThemeColors() {
    if (!container) return;
    const styles = getComputedStyle(container);
    tileColor = styles.getPropertyValue('--tile').trim() || tileColor;
    lineColor = styles.getPropertyValue('--tile-line').trim() || lineColor;
  }

  function draw(now = performance.now()) {
    const ctx = canvas?.getContext('2d');
    if (!ctx || !canvas) return;

    const { width, height } = box;
    if (width <= 0 || height <= 0) return;

    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const pixelWidth = Math.round(width * dpr);
    const pixelHeight = Math.round(height * dpr);
    if (canvas.width !== pixelWidth || canvas.height !== pixelHeight) {
      canvas.width = pixelWidth;
      canvas.height = pixelHeight;
    }

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);

    const cellWidth = width / cols;
    const cellHeight = height / rows;
    const total = cols * rows;

    // Pass one: every fully covered tile, as two batched draw calls.
    const fills = new Path2D();
    const lines = new Path2D();
    let covered = 0;

    for (let i = 0; i < total; i++) {
      if (settled.has(i) || animating.has(i)) continue;
      const x = (i % cols) * cellWidth;
      const y = Math.floor(i / cols) * cellHeight;
      fills.rect(x, y, cellWidth, cellHeight);
      // Inset by half a pixel so the stroke lands on the tile, not between
      // tiles: a revealed neighbour must leave no line behind.
      lines.rect(x + 0.5, y + 0.5, cellWidth - 1, cellHeight - 1);
      covered++;
    }

    if (covered > 0) {
      ctx.fillStyle = tileColor;
      ctx.fill(fills);
      ctx.strokeStyle = lineColor;
      ctx.lineWidth = 1;
      ctx.stroke(lines);
    }

    // Pass two: tiles mid-reveal, each with its own alpha and scale.
    let stillAnimating = false;
    for (const [index, startedAt] of animating) {
      const progress = Math.min(1, Math.max(0, (now - startedAt) / TILE_DURATION));
      if (progress >= 1) {
        settled.add(index);
        animating.delete(index);
        continue;
      }
      stillAnimating = true;

      // easeOutCubic: quick to depart, gentle to finish.
      const eased = 1 - Math.pow(1 - progress, 3);
      const alpha = 1 - eased;
      const shrink = eased * 0.5;

      const x = (index % cols) * cellWidth;
      const y = Math.floor(index / cols) * cellHeight;
      const insetX = cellWidth * shrink * 0.5;
      const insetY = cellHeight * shrink * 0.5;

      ctx.globalAlpha = alpha;
      ctx.fillStyle = tileColor;
      ctx.fillRect(
        x + insetX,
        y + insetY,
        cellWidth - insetX * 2,
        cellHeight - insetY * 2
      );
      ctx.globalAlpha = 1;
    }

    if (stillAnimating || animating.size > 0) {
      frame = requestAnimationFrame(draw);
    } else if (frame !== 0) {
      frame = 0;
      onanimationend?.();
    }
  }

  /**
   * Animates the given tiles away.
   *
   * Everything else in `revealed` is expected to have been snapped in already,
   * so the animation only ever shows the caller's own contribution.
   */
  export function animateReveal(tiles: number[]) {
    if (tiles.length === 0) return;

    if (prefersReducedMotion()) {
      for (const tile of tiles) settled.add(tile);
      draw();
      onanimationend?.();
      return;
    }

    const now = performance.now();
    const stagger = Math.min(STAGGER, MAX_TOTAL_STAGGER / Math.max(tiles.length, 1));
    tiles.forEach((tile, i) => {
      if (settled.has(tile)) return;
      animating.set(tile, now + i * stagger);
    });

    if (frame === 0) frame = requestAnimationFrame(draw);
  }

  /** Applies tiles with no animation — used for other members' progress. */
  export function snapTo(tiles: number[]) {
    for (const tile of tiles) {
      settled.add(tile);
      animating.delete(tile);
    }
    draw();
  }

  onMount(() => {
    readThemeColors();

    const observer = new ResizeObserver((entries) => {
      const rect = entries[0]?.contentRect;
      if (!rect) return;
      containerWidth = rect.width;
      containerHeight = rect.height;
    });
    if (container) {
      observer.observe(container);
      containerWidth = container.clientWidth;
      containerHeight = container.clientHeight;
    }

    const scheme = matchMedia('(prefers-color-scheme: dark)');
    const onSchemeChange = () => {
      readThemeColors();
      draw();
    };
    scheme.addEventListener('change', onSchemeChange);

    return () => {
      observer.disconnect();
      scheme.removeEventListener('change', onSchemeChange);
      if (frame !== 0) cancelAnimationFrame(frame);
    };
  });

  // Adopt the authoritative revealed set from the server. Tiles already
  // animating are left alone so a submit in flight is not cut short.
  $effect(() => {
    const incoming = new Set(revealed);
    for (const tile of incoming) {
      if (!animating.has(tile)) settled.add(tile);
    }
    // A rewind (an edited or deleted entry) re-covers tiles.
    for (const tile of settled) {
      if (!incoming.has(tile)) settled.delete(tile);
    }
    draw();
  });

  // Redraw on any geometry change.
  $effect(() => {
    void box;
    void cols;
    void rows;
    draw();
  });
</script>

<div class="board" bind:this={container}>
  <div
    class="stage"
    style={`left:${box.left}px; top:${box.top}px; width:${box.width}px; height:${box.height}px;`}
  >
    <img {src} alt="" draggable="false" />
    <canvas
      bind:this={canvas}
      style={`width:${box.width}px; height:${box.height}px;`}
      aria-hidden="true"
    ></canvas>
  </div>
</div>

<style>
  .board {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: #000;
  }

  .stage { position: absolute; }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: fill;
    user-select: none;
    -webkit-user-drag: none;
  }

  canvas {
    position: absolute;
    inset: 0;
    display: block;
  }
</style>
