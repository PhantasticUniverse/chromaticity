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
  import { untrack } from 'svelte';

  import { RENDERERS, type SceneKind, type SceneRendererControls } from './scene-renderers';
  import { fetchGpuFrame, fetchGpuRenderStatus } from '../ipc/render';
  import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';
  import { loadedSceneStore } from '../stores/loadedScene.svelte';

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
    /** Wave 2.9 SC.A — optional name-keyed control snapshot from the
     *  loaded scene's schema. Recognised common-axis keys:
     *  `speed` (Slider in `[0, 1]`, 0.5 nominal) pre-multiplies the
     *  renderer's time arg so `0.0` freezes and `1.0` runs at 2x;
     *  `intensity` (Slider in `[0, 1]`, 0.5 nominal) pre-multiplies
     *  the renderer's `lvl` arg with the same 2x scaling so the silent-
     *  audio fallback (`pulse = 0.6 + lvl * 0.4`) still has headroom;
     *  `tint` (Vec3 or Vec4 of `[r, g, b, (a)]`, channels in `[0, 1]`)
     *  draws a translucent overlay over the vignette post-pass at 20%
     *  of the supplied alpha. Other scene-specific controls flow to
     *  `scene-renderers.ts`, where renderer-owned mappings cover
     *  geometry, palette, scale, and audio-band axes without widening
     *  Perform. Missing keys default to nominal so scenes without
     *  these axes render identically to pre-control behaviour. */
    controls?: SceneRendererControls;
    /** When true, prefers live `wgpu` GPU frames from `render_gpu_frame` whenever
     *  the loaded `.chroma` scene ships a `main.glsl` shader, falling back
     *  seamlessly to the procedural Canvas2D renderer otherwise. */
    gpuLive?: boolean;
  };

  let {
    kind = 'rings',
    seed = 0,
    audio = true,
    dpr,
    ariaLabel,
    controls,
    gpuLive = false,
  }: Props = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let reduceMotion = $state(false);
  let gpuActive = $state(false);

  $effect(() => {
    if (!gpuLive) {
      gpuActive = false;
      return;
    }
    const sceneId = loadedSceneStore.snapshot?.id ?? null;
    if (!sceneId) {
      gpuActive = false;
      return;
    }
    let cancelled = false;
    void fetchGpuRenderStatus().then((status) => {
      if (!cancelled) {
        gpuActive = status !== null && status.state === 'active';
      }
    });
    return () => {
      cancelled = true;
    };
  });

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

    const resolveDpr = (explicitDpr: number | undefined): number =>
      explicitDpr ??
      (typeof window !== 'undefined' && typeof window.devicePixelRatio === 'number'
        ? window.devicePixelRatio
        : 1);

    const controlEnabled = (ctrls: SceneRendererControls | undefined, name: string): boolean => {
      const value = ctrls?.[name];
      return typeof value === 'number' && value >= 0.5;
    };

    let raf = 0;
    const resize = (bitmapDpr: number): void => {
      const r = c.getBoundingClientRect();
      // Wave 2.3 Q.1: only reassign `c.width` / `c.height` when the
      // computed bitmap size differs from the current one. Setting
      // the property always clears the bitmap (HTML spec) even when
      // the value is unchanged — without the guard, ResizeObserver
      // firing faster than rAF caused the AO-reported "flashing"
      // black canvas.
      const nw = Math.max(1, Math.round(r.width * (bitmapDpr || 1)));
      const nh = Math.max(1, Math.round(r.height * (bitmapDpr || 1)));
      if (c.width !== nw) c.width = nw;
      if (c.height !== nh) c.height = nh;
    };
    // Wave 2.3 Q.1: DPR defaults to the device-pixel-ratio so retina
    // displays render sharp. Consumers can still pass `dpr={1}` to
    // override. The prop read is untracked so dpr changes resize on
    // the next frame instead of tearing down the rAF setup effect.
    resize(untrack(() => resolveDpr(dpr)));

    // jsdom does not implement ResizeObserver in every Vitest config;
    // guard so tests don't blow up on construction.
    let ro: ResizeObserver | undefined;
    if (typeof ResizeObserver !== 'undefined') {
      ro = new ResizeObserver(() => resize(untrack(() => resolveDpr(dpr))));
      ro.observe(c);
    }

    const draw = (
      t: number,
      lvl: number,
      frameKind: SceneKind,
      frameSeed: number,
      ctrls?: SceneRendererControls,
    ): void => {
      // Wave 2.9 SC.A — pre-multiply common-axis controls at the
      // adapter boundary so the renderers keep a 6-arg public arity
      // while accepting optional control metadata via a rest param.
      // `speed` (0.5 nominal, 1.0 = 2x, 0.0 = frozen) scales `t`;
      // `intensity` (0.5 nominal) scales `lvl`. The `* 2` factor
      // maps the `[0, 1]` Slider range onto a `[0, 2]` multiplier so
      // the nominal 0.5 reproduces the pre-Wave-2.9 baseline.
      const speedRaw = ctrls?.speed;
      const speed = typeof speedRaw === 'number' ? speedRaw : 0.5;
      const effectiveT = t * (speed * 2);

      const intensityRaw = ctrls?.intensity;
      const intensity = typeof intensityRaw === 'number' ? intensityRaw : 0.5;
      const effectiveLvl = Math.max(0, Math.min(1, lvl * intensity * 2));

      const r = RENDERERS[frameKind] ?? RENDERERS.rings;
      const mirrorX =
        controlEnabled(ctrls, 'mirror') ||
        controlEnabled(ctrls, 'vert_mirror') ||
        controlEnabled(ctrls, 'vertical_mirror');
      const mirrorY =
        controlEnabled(ctrls, 'hor_mirror') || controlEnabled(ctrls, 'horizontal_mirror');
      if (mirrorX || mirrorY) {
        ctx.save();
        ctx.translate(mirrorX ? c.width : 0, mirrorY ? c.height : 0);
        ctx.scale(mirrorX ? -1 : 1, mirrorY ? -1 : 1);
        r(ctx, effectiveT, frameSeed, c.width, c.height, effectiveLvl, ctrls);
        ctx.restore();
      } else {
        r(ctx, effectiveT, frameSeed, c.width, c.height, effectiveLvl, ctrls);
      }
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

      // Wave 2.9 SC.A — `tint` overlay post-vignette so the colour
      // wash sits on top of the procedural pattern + the soft edge.
      // The `Vec3` shape carries RGB only (alpha defaults to 1.0);
      // the `Vec4` shape carries RGBA. Overlay alpha is scaled by 0.2
      // so a fully-saturated tint still reads as a translucent wash,
      // not an opaque cover. Missing or non-array `tint` keys skip
      // the overlay entirely (the pre-Wave-2.9 visual baseline).
      const tint = ctrls?.tint;
      if (Array.isArray(tint) && tint.length >= 3) {
        const tr = typeof tint[0] === 'number' ? tint[0] : 0;
        const tg = typeof tint[1] === 'number' ? tint[1] : 0;
        const tb = typeof tint[2] === 'number' ? tint[2] : 0;
        const taRaw = tint.length >= 4 && typeof tint[3] === 'number' ? tint[3] : 1.0;
        const overlayAlpha = Math.max(0, Math.min(1, taRaw * 0.2));
        if (overlayAlpha > 0) {
          const rr = Math.round(Math.max(0, Math.min(1, tr)) * 255);
          const gg = Math.round(Math.max(0, Math.min(1, tg)) * 255);
          const bb = Math.round(Math.max(0, Math.min(1, tb)) * 255);
          ctx.fillStyle = `rgba(${rr}, ${gg}, ${bb}, ${overlayAlpha})`;
          ctx.fillRect(0, 0, c.width, c.height);
        }
      }

      if (controlEnabled(ctrls, 'invert')) {
        ctx.save();
        ctx.globalCompositeOperation = 'difference';
        ctx.fillStyle = '#fff';
        ctx.fillRect(0, 0, c.width, c.height);
        ctx.restore();
      }
    };

    if (reduceMotion) {
      // Static frame — equivalent to a SceneStill emit.
      const frame = untrack(() => ({
        kind,
        seed,
        controls,
        dpr: resolveDpr(dpr),
      }));
      resize(frame.dpr);
      draw(frame.seed * 13.7, 0.5, frame.kind, frame.seed, frame.controls);
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
    let gpuInFlight = false;
    let lastGpuFrame: ImageData | null = null;
    let gpuOffscreen: HTMLCanvasElement | null = null;
    const GPU_MAX_W = 480;
    const GPU_MAX_H = 270;

    const loop = (): void => {
      const t = (performance.now() - start) / 1000;
      const frame = untrack(() => ({
        audio,
        kind,
        seed,
        controls,
        dpr: resolveDpr(dpr),
        snapshot: audioSnapshotStore.snapshot,
        useGpu: gpuLive && gpuActive,
      }));
      resize(frame.dpr);

      if (frame.useGpu && typeof ImageData !== 'undefined') {
        const targetW = Math.max(16, Math.min(GPU_MAX_W, Math.round(c.width)));
        const targetH = Math.max(16, Math.min(GPU_MAX_H, Math.round(c.height)));
        if (!gpuInFlight) {
          gpuInFlight = true;
          void fetchGpuFrame(targetW, targetH).then((pixels) => {
            gpuInFlight = false;
            if (pixels) {
              const clamped = new Uint8ClampedArray(pixels.byteLength);
              clamped.set(pixels);
              lastGpuFrame = new ImageData(clamped, targetW, targetH);
            } else {
              gpuActive = false;
              lastGpuFrame = null;
            }
          });
        }
        if (lastGpuFrame) {
          if (!gpuOffscreen && typeof document !== 'undefined') {
            gpuOffscreen = document.createElement('canvas');
          }
          if (gpuOffscreen) {
            if (
              gpuOffscreen.width !== lastGpuFrame.width ||
              gpuOffscreen.height !== lastGpuFrame.height
            ) {
              gpuOffscreen.width = lastGpuFrame.width;
              gpuOffscreen.height = lastGpuFrame.height;
            }
            const offCtx = gpuOffscreen.getContext('2d');
            if (offCtx) {
              offCtx.putImageData(lastGpuFrame, 0, 0);
              ctx.drawImage(gpuOffscreen, 0, 0, c.width, c.height);
              raf = requestAnimationFrame(loop);
              return;
            }
          }
        }
      } else {
        lastGpuFrame = null;
      }

      // Wave 2.2 Lane R.1 + Wave 2.3 Q.1 — prefer the live engine
      // snapshot when present (gate widened from "rms > 0 || peak > 0"
      // to "snapshot !== null" so silent-microphone audio still drives
      // the renderer instead of falling through to synthetic). The
      // synthetic sine-pulse only renders during the no-snapshot
      // startup window + the no-Tauri-context fallback.
      let rawLvl: number;
      if (frame.audio) {
        const live = frame.snapshot;
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
      draw(t, smLvl, frame.kind, frame.seed, frame.controls);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);

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
