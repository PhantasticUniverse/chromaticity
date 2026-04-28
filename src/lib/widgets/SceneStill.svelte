<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // W36 SceneStill widget — static thumbnail tile rendered via canvas-2d
  // procedural renderers ported from the v0.2-beta bundle's `shaders.jsx`
  // (see `./scene-renderers.ts` for the SceneKind family and the
  // `renderStill` helper). This is the bundle's `<SceneStill>` primitive
  // surface ported to Svelte 5; the live-animated `SceneCanvas` variant
  // (audio-pulsed requestAnimationFrame loop) is a later wave.
  //
  // Source of truth (port reference):
  //   chromaticity-planning/ui-predev-bundle-v0.2-beta/shaders.jsx
  //   `SceneStill` JSX + `renderStill` helper (file lines 264-320).
  //
  // Consumers (W36 PR1 wires SnapBar; later waves wire Library /
  // Playlist sidebar / Author scene-list / Perform scene rail):
  //   - `SnapBar.svelte` per-slot preview (replaces the W6 PR2 gradient
  //     surrogate; see `SnapBar.svelte` lines 24-27 for the deferral note
  //     this widget closes).
  //
  // The widget is a thin Svelte wrapper around `<canvas>`: width / height
  // size the bitmap, `kind` + `seed` parameterise the renderer. A
  // `$effect` re-draws when any of those three change. There is no
  // animation loop and no resize observer — this is a one-shot still
  // image. The bundle's vignette post-pass is part of `renderStill` and
  // is therefore inherited automatically.
  import { renderStill, type SceneKind } from './scene-renderers';

  type Props = {
    kind?: SceneKind;
    seed?: number;
    width?: number;
    height?: number;
    /**
     * Optional ARIA description. Default `presentation` because the
     * canvas is decorative — the operator reads the label/identifier
     * from the surrounding cell, not the thumbnail itself.
     */
    ariaLabel?: string;
  };

  let { kind = 'rings', seed = 0, width = 200, height = 120, ariaLabel }: Props = $props();

  let canvas: HTMLCanvasElement | undefined = $state();

  // Re-render whenever the canvas mounts or the renderer-controlling
  // props (kind / seed / width / height) change. `width` / `height` set
  // the canvas bitmap before the renderer runs so the routine sees the
  // right `(w, h)`.
  $effect(() => {
    if (!canvas) return;
    canvas.width = width;
    canvas.height = height;
    renderStill(canvas, kind, seed);
  });
</script>

<canvas
  bind:this={canvas}
  class="scene-still"
  {width}
  {height}
  aria-label={ariaLabel}
  role={ariaLabel ? 'img' : 'presentation'}
></canvas>

<style>
  .scene-still {
    display: block;
    width: 100%;
    height: 100%;
    /* Render the bitmap with a soft pixel resampling (default) — the
     * vignette in `renderStill` already cinematises edges, so we don't
     * apply additional CSS filters or blends here. */
  }
</style>
