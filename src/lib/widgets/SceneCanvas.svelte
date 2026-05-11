<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Shared-primitive backfill — `SceneCanvas` (live-animated variant)
  // from `chromaticity-planning/ui-predev-bundle-v0.2-beta/shaders.jsx`
  // lines 278-310. Sibling to the W36 `SceneStill` widget — they share
  // the `RENDERERS` map (`scene-renderers.ts`) but `SceneCanvas` adds
  // a `requestAnimationFrame` loop, a `ResizeObserver` for DPR-aware
  // bitmap sizing, a vignette + audio-pulse post-pass, and an `audio`
  // toggle that drives a synthetic level-pulse in `[0, 1]`.
  //
  // Bundle parity (verbatim signature):
  //   <SceneCanvas kind="rings" seed={4} audio dpr={1} />
  //
  // Wave 2.2 Lane R.1 widens the `audio` arm to read
  // `audioSnapshotStore.snapshot?.rms` when a live snapshot is
  // present (BPM-driven engine producer per ADR-019). When the
  // snapshot is `null` — startup window before the first audio
  // tick fires, headless / no-input-device fallback, vite-dev mode
  // without Tauri — the widget keeps its synthetic
  // sine-pulse for visual continuity. `prefers-reduced-motion:
  // reduce` still short-circuits the rAF loop and pins one frame
  // (t=0, audio=0.5 — same baseline as `SceneStill`'s
  // `renderStill`) so users with motion preferences see a
  // recognisable thumbnail surface.
  //
  // Consumers (Perform preview hero, Library hover preview, Author
  // live-preview tile) can swap from `SceneStill` to `SceneCanvas`
  // when they want a moving image. The widget is a thin Svelte
  // wrapper around `<canvas>`: the `$effect` owns the rAF +
  // ResizeObserver and tears them down on unmount.
  import { RENDERERS, type SceneKind } from './scene-renderers';
  import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';

  type Props = {
    kind?: SceneKind;
    seed?: number;
    /** When true, the renderer receives a synthetic audio-level pulse;
     *  when false, the level is fixed at 0.3 (the bundle's no-audio
     *  default). */
    audio?: boolean;
    /** Device-pixel-ratio multiplier for the bitmap. Default 1. */
    dpr?: number;
    /** Optional ARIA description. Default `presentation` (decorative). */
    ariaLabel?: string;
  };

  let { kind = 'rings', seed = 0, audio = true, dpr = 1, ariaLabel }: Props = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let reduceMotion = $state(false);

  // Wave 2.2 Lane R.1 — subscribe to the live audio IPC seam on
  // mount so the rAF loop's per-frame `lvl` read picks up real RMS
  // when available. The store call is idempotent (Wave 1c-α
  // precedent) so multiple SceneCanvas mounts share one listener.
  $effect(() => {
    if (!audio) return;
    void audioSnapshotStore.subscribe();
  });

  $effect(() => {
    if (typeof window === 'undefined') return;
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    reduceMotion = mq.matches;
    const onChange = (e: MediaQueryListEvent): void => {
      reduceMotion = e.matches;
    };
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  $effect(() => {
    if (!canvas) return;
    const c = canvas;
    const ctx = c.getContext('2d');
    if (!ctx) return;

    let raf = 0;
    const resize = (): void => {
      const r = c.getBoundingClientRect();
      c.width = Math.max(1, Math.round(r.width * (dpr || 1)));
      c.height = Math.max(1, Math.round(r.height * (dpr || 1)));
    };
    resize();

    // jsdom does not implement ResizeObserver in every Vitest config;
    // guard so tests don't blow up on construction.
    let ro: ResizeObserver | undefined;
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver(resize);
      ro.observe(c);
    }

    const draw = (t: number, lvl: number): void => {
      const r = RENDERERS[kind] ?? RENDERERS.rings;
      r(ctx, t, seed, c.width, c.height, Math.max(0, Math.min(1, lvl)));
      // Vignette + scanline post-pass per the bundle.
      const vg = ctx.createRadialGradient(
        c.width / 2,
        c.height / 2,
        Math.min(c.width, c.height) * 0.35,
        c.width / 2,
        c.height / 2,
        Math.max(c.width, c.height) * 0.75,
      );
      vg.addColorStop(0, 'transparent');
      vg.addColorStop(1, 'rgba(0,0,0,0.65)');
      ctx.fillStyle = vg;
      ctx.fillRect(0, 0, c.width, c.height);
    };

    if (reduceMotion) {
      // Static frame — equivalent to a SceneStill emit.
      draw(seed * 13.7, 0.5);
      return () => {
        ro?.disconnect();
      };
    }

    const start = performance.now();
    const loop = (): void => {
      const t = (performance.now() - start) / 1000;
      // Wave 2.2 Lane R.1 — prefer the live engine RMS when a
      // snapshot is present; otherwise fall back to the bundle's
      // synthetic pulse so the no-snapshot startup window + the
      // no-Tauri-context fallback still animate.
      let lvl: number;
      if (audio) {
        const live = audioSnapshotStore.snapshot;
        if (live && (live.rms > 0 || live.peak > 0)) {
          // RMS is already in [0, 1] from the engine producer; clamp
          // for safety since the wire is `f32`. Add a small peak
          // contribution so transient hits still register.
          lvl = Math.max(0, Math.min(1, live.rms + live.peak * 0.25));
        } else {
          lvl = (Math.sin(t * 6.28 * 0.8) * 0.5 + 0.5) * 0.7 + Math.sin(t * 6.28 * 2.3) * 0.2;
        }
      } else {
        lvl = 0.3;
      }
      draw(t, lvl);
      raf = requestAnimationFrame(loop);
    };
    loop();

    return () => {
      cancelAnimationFrame(raf);
      ro?.disconnect();
    };
  });
</script>

<canvas
  bind:this={canvas}
  class="scene-canvas"
  aria-label={ariaLabel}
  role={ariaLabel ? 'img' : 'presentation'}
></canvas>

<style>
  .scene-canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
