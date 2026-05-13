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
  // **Wave 2.3 Q.1 stress-test fixes.** The AO smoke-test of Wave 2.2
  // R.1 reported the Perform preview pane (driving the `bars`
  // SceneKind for `library/demo-spectrum.chroma`) as "mostly black
  // with a static version flashing on and off". Four narrowings:
  //
  //   1. **ResizeObserver flicker** — the bundle's `resize()`
  //      unconditionally reassigns `c.width` / `c.height` on every
  //      ResizeObserver tick. Per the HTML spec, ANY write to
  //      `canvas.width` clears the bitmap to transparent — even when
  //      the new value equals the existing value. When the layout
  //      reflows faster than rAF (initial mount on macOS + window-
  //      decoration animations), the bitmap is repeatedly cleared
  //      between draws. The fix guards the write behind a
  //      "value actually changed" check.
  //   2. **DPR=1 default on retina** — the bundle hardcoded `dpr=1`
  //      because its React surface ran in a Storybook-like demo. On
  //      a 2x macOS display the bitmap is half the panel's physical
  //      pixel density and looks visibly soft. Default to
  //      `window.devicePixelRatio ?? 1` so retina displays render
  //      sharp out of the box; consumers can still pass an explicit
  //      `dpr` to override.
  //   3. **Audio gate too narrow** — the Wave 2.2 R.1 gate
  //      `live && (live.rms > 0 || live.peak > 0)` falls through to
  //      the synthetic pulse when the live producer is running but
  //      the microphone is silent. The intent is "use live when the
  //      IPC seam is up, synthetic only as a startup fallback"; the
  //      fix widens the gate to `live !== null`.
  //   4. **Vignette over-darkening** — the bundle's 0.65 outer-stop
  //      darkens the edges of wide preview panes into unreadability.
  //      Soften to 0.45 to match the bundle's intent of a subtle
  //      CRT-style vignette.
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
    /** Device-pixel-ratio multiplier for the bitmap. Defaults to
     *  `window.devicePixelRatio ?? 1` so retina displays render sharp.
     *  Pass an explicit value to override (e.g., `dpr={1}` for
     *  performance on dense canvases). */
    dpr?: number;
    /** Optional ARIA description. Default `presentation` (decorative). */
    ariaLabel?: string;
  };

  let { kind = 'rings', seed = 0, audio = true, dpr, ariaLabel }: Props = $props();

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

    // Wave 2.3 Q.1: DPR defaults to the device-pixel-ratio so retina
    // displays render sharp. Consumers can still pass `dpr={1}` to
    // override. `typeof window` guard for SSR / Vitest jsdom where
    // `window.devicePixelRatio` is missing or `undefined`.
    const effectiveDpr =
      dpr ??
      (typeof window !== 'undefined' && typeof window.devicePixelRatio === 'number'
        ? window.devicePixelRatio
        : 1);

    let raf = 0;
    const resize = (): void => {
      const r = c.getBoundingClientRect();
      // Wave 2.3 Q.1: only reassign `c.width` / `c.height` when the
      // computed bitmap size differs from the current one. Setting
      // the property always clears the bitmap (HTML spec) even when
      // the value is unchanged — without the guard, ResizeObserver
      // firing faster than rAF caused the AO-reported "flashing"
      // black canvas.
      const nw = Math.max(1, Math.round(r.width * (effectiveDpr || 1)));
      const nh = Math.max(1, Math.round(r.height * (effectiveDpr || 1)));
      if (c.width !== nw) c.width = nw;
      if (c.height !== nh) c.height = nh;
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
      // Vignette + scanline post-pass per the bundle. Wave 2.3 Q.1:
      // softened from 0.65 → 0.45 to keep wide preview panes
      // readable at the edges.
      const vg = ctx.createRadialGradient(
        c.width / 2,
        c.height / 2,
        Math.min(c.width, c.height) * 0.35,
        c.width / 2,
        c.height / 2,
        Math.max(c.width, c.height) * 0.75,
      );
      vg.addColorStop(0, 'transparent');
      vg.addColorStop(1, 'rgba(0,0,0,0.45)');
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
    // Wave 2.8 Q.1 — persistent `lvl` smoothing state. Without an EMA
    // the per-frame `lvl` reads bin-level jitter from the engine
    // producer (background mic noise reports multi-millisecond
    // bursts), which fed straight into renderer axes like ring stroke
    // width, particle radius, and lightness boosts — preview tiles
    // appeared to fidget at silent input. The EMA tracks real audio
    // onsets in ~80 ms but discards sub-2-frame noise.
    let smLvl = 0;
    const LVL_EMA = 0.25;
    const loop = (): void => {
      const t = (performance.now() - start) / 1000;
      // Wave 2.2 Lane R.1 + Wave 2.3 Q.1 — prefer the live engine
      // snapshot when present (gate widened from "rms > 0 || peak > 0"
      // to "snapshot !== null" so silent-microphone audio still drives
      // the renderer instead of falling through to synthetic). The
      // synthetic sine-pulse only renders during the no-snapshot
      // startup window + the no-Tauri-context fallback.
      let rawLvl: number;
      if (audio) {
        const live = audioSnapshotStore.snapshot;
        if (live !== null) {
          // RMS is already in [0, 1] from the engine producer; clamp
          // for safety since the wire is `f32`. Add a small peak
          // contribution so transient hits still register.
          rawLvl = Math.max(0, Math.min(1, live.rms + live.peak * 0.25));
        } else {
          rawLvl = (Math.sin(t * 6.28 * 0.8) * 0.5 + 0.5) * 0.7 + Math.sin(t * 6.28 * 2.3) * 0.2;
        }
      } else {
        rawLvl = 0.3;
      }
      smLvl = smLvl + (rawLvl - smLvl) * LVL_EMA;
      draw(t, smLvl);
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
