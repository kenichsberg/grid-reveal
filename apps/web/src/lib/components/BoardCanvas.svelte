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

  /**
   * A revealed tile is a brick that breaks.
   *
   * It fractures in place for a moment — cracks drawn across its face — then
   * bursts into fragments that fly outward under gravity and fade as they
   * fall. The fragments are a plain particle system in canvas space, no longer
   * tied to the grid once they are airborne.
   */

  /** How long a tile sits cracked before it comes apart, in ms. */
  const CRACK_MS = 110;
  /** Delay between successive tiles cracking, in ms. */
  const STAGGER = 18;
  /** Keeps a big simultaneous reveal from dragging on; tiles overlap instead. */
  const MAX_TOTAL_STAGGER = 1400;

  /** Fragment lifetime, in ms, before and after jitter. */
  const FRAGMENT_LIFE = 1000;
  const FRAGMENT_LIFE_JITTER = 500;
  /**
   * Fragments per axis, so a tile breaks into GRID x GRID pieces.
   *
   * Reduced automatically for small cells and for large reveals; see
   * `fragmentGridFor`.
   */
  const FRAGMENT_GRID = 3;
  /**
   * The ceiling on live fragments.
   *
   * A reveal of several hundred tiles would otherwise spawn tens of thousands
   * of pieces. Past this the pieces per tile drop rather than the frame rate.
   */
  const MAX_FRAGMENTS = 2600;
  /**
   * The smallest a drawn chip may be, in px.
   *
   * Fragment size is otherwise a cell divided by the fragment grid, which on a
   * dense board means a two-pixel crumb nobody can see. Below this floor the
   * pieces are drawn larger than their lattice spacing: they overlap at the
   * moment of the break, which reads as a solid brick coming apart, and
   * separate within a few frames.
   */
  const MIN_PIECE_PX = 5.5;
  /**
   * The smallest cell size, in px, that physics is scaled from.
   *
   * Speed, lift and gravity are all multiples of a cell, so on a dense board
   * the debris would travel a handful of pixels and look like it was dropped
   * rather than thrown. Small cells borrow a larger cell's energy.
   */
  const MIN_BURST_PX = 30;
  /** Below this cell size in px, the lattice itself stops being meaningful. */
  const MIN_FRAGMENT_PX = 1.6;

  /** Outward speed, as multiples of a cell per second. */
  const SPEED_MIN = 2.4;
  const SPEED_MAX = 6.2;
  /** Upward kick on the way out, as multiples of a cell per second. */
  const LIFT = 2.1;
  /** Downward acceleration, as multiples of a cell per second squared. */
  const GRAVITY = 11;
  /** Radians per second a fragment tumbles, at most. */
  const MAX_SPIN = 7;
  /** Rotating each piece costs a transform; below this it is not visible. */
  const SPIN_THRESHOLD_PX = 6.5;

  type Fragment = {
    /** Centre, in canvas pixels. */
    x: number;
    y: number;
    w: number;
    h: number;
    vx: number;
    vy: number;
    gravity: number;
    rot: number;
    vrot: number;
    /** Remaining life in ms, counting down from `life`. */
    left: number;
    life: number;
    shade: string;
  };

  let container = $state<HTMLDivElement | null>(null);
  let canvas = $state<HTMLCanvasElement | null>(null);

  let containerWidth = $state(0);
  let containerHeight = $state(0);

  /** Tiles revealed and fully drawn as absent. */
  let settled = new Set<number>();
  /**
   * Tiles mid-fracture: index -> the timestamp at which it starts cracking.
   *
   * A tile leaves this map the moment it shatters; its fragments live on in
   * `fragments`, which is why both have to be empty before the loop stops.
   */
  let animating = new Map<number, number>();
  /** Airborne fragments, in canvas pixels. */
  let fragments: Fragment[] = [];
  /** How many pieces each tile of this reveal breaks into, per axis. */
  let fragmentGrid = FRAGMENT_GRID;
  let frame = 0;
  /** Timestamp of the previous frame, for the fragment integration step. */
  let lastFrameAt = 0;

  let tileColor = '#8b8f96';
  let lineColor = '#74787f';
  /** Fragment shades, from darkest to lightest chip of a broken tile. */
  let shades = ['#8b8f96'];

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
    shades = buildShades(tileColor);
  }

  /**
   * A palette of chips around the tile colour.
   *
   * Fragments all in one flat colour read as a dissolving block; a spread of
   * light and dark pieces reads as broken material catching the light.
   * Pre-computed as strings because `fillStyle` is set per fragment.
   */
  function buildShades(color: string): string[] {
    const rgb = parseColor(color);
    if (!rgb) return [color];
    const [r, g, b] = rgb;
    const clamp = (n: number) => Math.max(0, Math.min(255, Math.round(n)));
    return [0.62, 0.78, 0.92, 1.06, 1.22, 1.4].map(
      (k) => `rgb(${clamp(r * k)} ${clamp(g * k)} ${clamp(b * k)})`
    );
  }

  /** Handles the `#rgb`, `#rrggbb`, and `rgb()` forms a computed style yields. */
  function parseColor(color: string): [number, number, number] | null {
    const hex = color.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
    if (hex) {
      const digits =
        hex[1].length === 3
          ? hex[1]
              .split('')
              .map((c) => c + c)
              .join('')
          : hex[1];
      return [
        parseInt(digits.slice(0, 2), 16),
        parseInt(digits.slice(2, 4), 16),
        parseInt(digits.slice(4, 6), 16)
      ];
    }
    const parts = color.match(/-?\d+(\.\d+)?/g);
    if (parts && parts.length >= 3) {
      return [Number(parts[0]), Number(parts[1]), Number(parts[2])];
    }
    return null;
  }

  /**
   * Cheap deterministic noise from a tile index.
   *
   * Crack angles come from this rather than from `Math.random`, so a cracking
   * tile keeps the same fracture pattern on every frame of its crack phase
   * without anything being stored per tile.
   */
  function noise(index: number, salt: number): number {
    const n = Math.sin(index * 127.1 + salt * 311.7) * 43758.5453;
    return n - Math.floor(n);
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

    // Pass two: tiles mid-fracture. Still whole, but visibly cracking, and
    // brightening as the break approaches.
    for (const [index, startedAt] of [...animating]) {
      const progress = (now - startedAt) / CRACK_MS;
      if (progress >= 1) {
        shatter(index, cellWidth, cellHeight);
        animating.delete(index);
        settled.add(index);
        continue;
      }
      if (progress <= 0) continue;

      const x = (index % cols) * cellWidth;
      const y = Math.floor(index / cols) * cellHeight;

      ctx.fillStyle = tileColor;
      ctx.fillRect(x, y, cellWidth, cellHeight);

      // The fracture: hairlines spreading from the centre as the tile gives
      // way. Clipped to the tile, because a crack that runs past its own edge
      // stops reading as a crack and starts reading as a scrawl.
      const centreX = x + cellWidth / 2;
      const centreY = y + cellHeight / 2;
      const reach = Math.min(cellWidth, cellHeight) / 2;

      ctx.save();
      ctx.beginPath();
      ctx.rect(x, y, cellWidth, cellHeight);
      ctx.clip();

      ctx.strokeStyle = lineColor;
      ctx.globalAlpha = 0.5 + progress * 0.5;
      ctx.lineWidth = Math.max(0.6, Math.min(cellWidth, cellHeight) * 0.035);
      ctx.beginPath();
      for (let c = 0; c < 6; c++) {
        const angle = noise(index, c) * Math.PI * 2;
        const length = reach * (0.7 + noise(index, c + 10) * 0.6) * progress;
        // A kink partway along, so the line breaks rather than radiating
        // perfectly straight.
        const bend = angle + (noise(index, c + 20) - 0.5) * 0.9;
        ctx.moveTo(centreX, centreY);
        ctx.lineTo(
          centreX + Math.cos(angle) * length * 0.55,
          centreY + Math.sin(angle) * length * 0.55
        );
        ctx.lineTo(
          centreX + Math.cos(bend) * length,
          centreY + Math.sin(bend) * length
        );
      }
      ctx.stroke();
      ctx.restore();
      ctx.globalAlpha = 1;
    }

    // Pass three: the fragments themselves.
    const dt = lastFrameAt === 0 ? 0 : Math.min(0.05, (now - lastFrameAt) / 1000);
    lastFrameAt = now;
    drawFragments(ctx, dt);

    if (animating.size > 0 || fragments.length > 0) {
      frame = requestAnimationFrame(draw);
    } else if (frame !== 0) {
      frame = 0;
      lastFrameAt = 0;
      onanimationend?.();
    }
  }

  /**
   * Integrates and draws every live fragment, dropping the dead ones.
   *
   * Compacts in place rather than filtering into a new array: this runs on
   * every frame of every reveal, and the array can hold a couple of thousand
   * pieces.
   */
  function drawFragments(ctx: CanvasRenderingContext2D, dt: number) {
    let live = 0;
    for (let i = 0; i < fragments.length; i++) {
      const f = fragments[i];

      f.left -= dt * 1000;
      if (f.left <= 0) continue;

      f.vy += f.gravity * dt;
      f.x += f.vx * dt;
      f.y += f.vy * dt;
      f.rot += f.vrot * dt;

      // Opaque while it flies, fading only over the last third, so a fragment
      // reads as a solid chip rather than a puff of smoke.
      const remaining = f.left / f.life;
      ctx.globalAlpha = remaining > 0.33 ? 1 : remaining / 0.33;
      ctx.fillStyle = f.shade;

      if (f.w >= SPIN_THRESHOLD_PX) {
        ctx.save();
        ctx.translate(f.x, f.y);
        ctx.rotate(f.rot);
        ctx.fillRect(-f.w / 2, -f.h / 2, f.w, f.h);
        ctx.restore();
      } else {
        // Too small for the tumble to register, so skip the transform and pay
        // one fillRect instead of a save/translate/rotate/restore.
        ctx.fillRect(f.x - f.w / 2, f.y - f.h / 2, f.w, f.h);
      }

      fragments[live++] = f;
    }
    fragments.length = live;
    ctx.globalAlpha = 1;
  }

  /** How finely a tile should break, given its size and the reveal's size. */
  function fragmentGridFor(tileCount: number, cellWidth: number, cellHeight: number): number {
    const smallest = Math.min(cellWidth, cellHeight);
    let grid = FRAGMENT_GRID;
    // Never produce sub-pixel crumbs.
    while (grid > 1 && smallest / grid < MIN_FRAGMENT_PX) grid--;
    // Nor more pieces in total than the budget allows.
    while (grid > 1 && tileCount * grid * grid > MAX_FRAGMENTS) grid--;
    return grid;
  }

  /** Breaks one tile into fragments and throws them outward. */
  function shatter(index: number, cellWidth: number, cellHeight: number) {
    const grid = fragmentGrid;
    // Lattice spacing: where the pieces start. Separate from their drawn size,
    // which has a floor, so a dense board breaks into chips that can be seen.
    const pieceWidth = cellWidth / grid;
    const pieceHeight = cellHeight / grid;
    const drawWidth = Math.max(pieceWidth, MIN_PIECE_PX);
    const drawHeight = Math.max(pieceHeight, MIN_PIECE_PX);
    // Physics is scaled from a cell, floored so small cells still throw their
    // debris a visible distance.
    const cell = Math.max(Math.max(cellWidth, cellHeight), MIN_BURST_PX);

    const originX = (index % cols) * cellWidth;
    const originY = Math.floor(index / cols) * cellHeight;
    const centreX = originX + cellWidth / 2;
    const centreY = originY + cellHeight / 2;

    for (let gy = 0; gy < grid; gy++) {
      for (let gx = 0; gx < grid; gx++) {
        if (fragments.length >= MAX_FRAGMENTS) return;

        const x = originX + (gx + 0.5) * pieceWidth;
        const y = originY + (gy + 0.5) * pieceHeight;

        // Outward from the tile's own centre, so the pieces separate instead
        // of travelling as a block. The single-piece case has no direction of
        // its own, so it gets a random one.
        let dx = x - centreX;
        let dy = y - centreY;
        const length = Math.hypot(dx, dy);
        if (length < 0.0001) {
          const angle = Math.random() * Math.PI * 2;
          dx = Math.cos(angle);
          dy = Math.sin(angle);
        } else {
          dx /= length;
          dy /= length;
        }

        const speed = cell * (SPEED_MIN + Math.random() * (SPEED_MAX - SPEED_MIN));
        const life = FRAGMENT_LIFE + Math.random() * FRAGMENT_LIFE_JITTER;

        fragments.push({
          x,
          y,
          // A touch of size jitter, so the break looks uneven.
          w: drawWidth * (0.7 + Math.random() * 0.45),
          h: drawHeight * (0.7 + Math.random() * 0.45),
          vx: dx * speed + (Math.random() - 0.5) * cell,
          vy: dy * speed - cell * LIFT * (0.6 + Math.random() * 0.8),
          gravity: cell * GRAVITY,
          rot: Math.random() * Math.PI,
          vrot: (Math.random() - 0.5) * 2 * MAX_SPIN,
          left: life,
          life,
          shade: shades[(Math.random() * shades.length) | 0]
        });
      }
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

    // Decided once for the whole reveal rather than per tile, so every tile in
    // one submission breaks the same way.
    const { width, height } = box;
    fragmentGrid = fragmentGridFor(
      tiles.length,
      width / cols,
      height / rows
    );

    tiles.forEach((tile, i) => {
      if (settled.has(tile)) return;
      animating.set(tile, now + i * stagger);
    });

    if (frame === 0) {
      lastFrameAt = 0;
      frame = requestAnimationFrame(draw);
    }
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
