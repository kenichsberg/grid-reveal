<script lang="ts">
  /**
   * Shown whenever a screen is fully revealed — for every member, on every
   * visit, as specified. Dismissible for the visit so it never blocks the
   * finished picture.
   *
   * The confetti is a few dozen lines on a canvas rather than a dependency;
   * the project already draws on canvas, so this adds nothing to the bundle.
   */
  import { onMount } from 'svelte';
  import { formatMinutes } from '$lib/format';

  type Props = {
    title: string;
    totalMinutes: number;
    ondismiss: () => void;
  };

  let { title, totalMinutes, ondismiss }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);

  const COLORS = ['#1f6feb', '#4cc38a', '#f5c518', '#ef6351', '#a78bfa'];
  const COUNT = 140;
  const DURATION = 4200;

  type Piece = {
    x: number;
    y: number;
    vx: number;
    vy: number;
    size: number;
    rotation: number;
    spin: number;
    color: string;
  };

  onMount(() => {
    const reduced =
      typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (reduced || !canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    let width = 0;
    let height = 0;

    const resize = () => {
      if (!canvas) return;
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.round(width * dpr);
      canvas.height = Math.round(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    window.addEventListener('resize', resize);

    const pieces: Piece[] = Array.from({ length: COUNT }, () => ({
      x: Math.random() * width,
      y: -20 - Math.random() * height * 0.5,
      vx: (Math.random() - 0.5) * 1.4,
      vy: 1.6 + Math.random() * 2.6,
      size: 5 + Math.random() * 7,
      rotation: Math.random() * Math.PI,
      spin: (Math.random() - 0.5) * 0.18,
      color: COLORS[Math.floor(Math.random() * COLORS.length)]
    }));

    const started = performance.now();
    let frame = requestAnimationFrame(function tick(now) {
      const elapsed = now - started;
      ctx.clearRect(0, 0, width, height);

      // Fade the whole burst out rather than stopping abruptly.
      const fade = elapsed > DURATION - 900 ? Math.max(0, (DURATION - elapsed) / 900) : 1;

      for (const piece of pieces) {
        piece.x += piece.vx;
        piece.y += piece.vy;
        piece.vy += 0.015;
        piece.rotation += piece.spin;

        if (piece.y > height + 20) {
          piece.y = -20;
          piece.x = Math.random() * width;
        }

        ctx.save();
        ctx.translate(piece.x, piece.y);
        ctx.rotate(piece.rotation);
        ctx.globalAlpha = fade;
        ctx.fillStyle = piece.color;
        ctx.fillRect(-piece.size / 2, -piece.size / 4, piece.size, piece.size / 2);
        ctx.restore();
      }

      if (elapsed < DURATION) {
        frame = requestAnimationFrame(tick);
      } else {
        ctx.clearRect(0, 0, width, height);
      }
    });

    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('resize', resize);
    };
  });
</script>

<canvas bind:this={canvas} class="confetti" aria-hidden="true"></canvas>

<div class="banner" role="status">
  <div class="card">
    <p class="eyebrow">Complete</p>
    <h2>{title}</h2>
    <p class="total">{formatMinutes(totalMinutes)} logged together</p>
    <button class="btn btn-sm" onclick={ondismiss}>View the picture</button>
  </div>
</div>

<style>
  .confetti {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: 14;
    pointer-events: none;
  }

  .banner {
    position: absolute;
    inset: 0;
    z-index: 15;
    display: grid;
    place-items: center;
    pointer-events: none;
    padding: 16px;
  }

  .card {
    pointer-events: auto;
    text-align: center;
    padding: 1.4rem 1.8rem;
    border-radius: 16px;
    background: rgb(0 0 0 / 0.72);
    backdrop-filter: blur(14px);
    border: 1px solid rgb(255 255 255 / 0.16);
    color: #fff;
    box-shadow: 0 20px 60px rgb(0 0 0 / 0.45);
    animation: rise 420ms cubic-bezier(0.2, 0.9, 0.3, 1);
  }

  .eyebrow {
    margin: 0 0 0.2rem;
    font-size: 0.72rem;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: rgb(255 255 255 / 0.65);
  }

  h2 { margin: 0 0 0.25rem; font-size: 1.35rem; }

  .total { margin: 0 0 1rem; color: rgb(255 255 255 / 0.8); font-size: 0.92rem; }

  .card .btn {
    background: rgb(255 255 255 / 0.14);
    border-color: rgb(255 255 255 / 0.25);
    color: #fff;
  }
  .card .btn:hover { background: rgb(255 255 255 / 0.24); }

  @keyframes rise {
    from { opacity: 0; transform: translateY(14px) scale(0.97); }
    to { opacity: 1; transform: none; }
  }
</style>
