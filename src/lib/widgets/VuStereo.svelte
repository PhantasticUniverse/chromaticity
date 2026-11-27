<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave 2.1 Lane B.2: widened with an optional `snapshot` prop so the
  // top-bar HUD mount can drive levels from the live audio uniform
  // store. The payload's `rms` is mono — we mirror it to both L and R
  // (with a tiny offset for visual separation) until a stereo
  // uniform-snapshot extension exists. Without a snapshot the widget
  // falls back to its prior synthetic rAF generator, preserving every
  // existing call-site's behaviour.
  //
  // Wave 2.8 Q.1: the live-snapshot and synthetic paths now share one
  // rAF loop with peak-hold + decay + level-EMA smoothing. Previously
  // the live-snapshot branch wrote `peakL = liveSnapshot.peak` raw on
  // every ~60 Hz engine tick, which made the white peak-marker dart
  // around — visually a flashing meter. The shared rAF + peak-hold +
  // EMA pattern fixes that while keeping the synthetic fallback intact.
  // `untrack` shields the rAF closure from Svelte 5 reactivity that
  // would otherwise re-run the effect on every snapshot write.
  import { untrack } from 'svelte';

  import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';
  import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';

  let {
    live = true,
    width = 100,
    snapshot = undefined,
  }: {
    live?: boolean;
    width?: number;
    /** Optional explicit snapshot for tests; when omitted the widget
     * reads from the module-singleton audioSnapshotStore. */
    snapshot?: AudioUniformSnapshotPayload | null;
  } = $props();

  const FROZEN_L = 0.6;
  const FROZEN_R = 0.55;

  let lvlL = $state(FROZEN_L);
  let lvlR = $state(FROZEN_R);
  let peakL = $state(FROZEN_L);
  let peakR = $state(FROZEN_R);

  $effect(() => {
    void audioSnapshotStore.subscribe();
  });

  // Effective snapshot: explicit prop wins, otherwise the live store.
  const liveSnapshot = $derived(snapshot ?? audioSnapshotStore.snapshot);

  function clamp(v: number): number {
    return Math.max(0, Math.min(0.995, v));
  }

  $effect(() => {
    const reduceMotion =
      typeof window !== 'undefined' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;

    if (!live || reduceMotion) {
      lvlL = FROZEN_L;
      lvlR = FROZEN_R;
      peakL = FROZEN_L;
      peakR = FROZEN_R;
      return;
    }

    // Wave 2.8 Q.1 — both live-snapshot and synthetic paths share a
    // single rAF loop with peak-hold + decay state. Prior code wrote
    // `peakL = liveSnapshot.peak` raw each time the snapshot updated,
    // which caused the peak marker line to dart with every engine tick
    // (~60 Hz) — visually that looks like the meter is flashing. With
    // peak-hold + decay the marker tracks the loudest recent peak and
    // glides down smoothly, matching the synthetic-path behaviour the
    // bundle baked in.
    //
    // The level (`--lvl`) is smoothed with a brief EMA so the gradient
    // stripe doesn't fidget on tiny RMS jitter (background mic noise
    // reports ~0.005-0.01 RMS even with no audible signal — without
    // smoothing the stripe width oscillates visibly).
    let frame = 0;
    const start = performance.now();
    let pL = 0;
    let pR = 0;
    let smL = 0;
    let smR = 0;
    const LEVEL_EMA = 0.25; // higher = more responsive, lower = more glide

    const tick = (now: number) => {
      let rawL: number;
      let rawR: number;
      let peakIn: number;
      // Untrack the live-snapshot read so $effect doesn't re-fire on
      // every engine tick (which would tear down + recreate the rAF
      // loop ~60x per second).
      const liveNow = untrack(() => liveSnapshot);
      if (liveNow) {
        // Live snapshot: mono rms / peak mirrored to both rows with a
        // tiny offset so the two rows aren't visually identical.
        rawL = clamp(liveNow.rms);
        rawR = clamp(liveNow.rms * 0.94);
        peakIn = clamp(liveNow.peak);
      } else {
        const t = (now - start) / 1000;
        rawL = Math.abs(Math.sin(t * 1.7)) * 0.6 + Math.abs(Math.sin(t * 5)) * 0.3;
        rawR = Math.abs(Math.sin(t * 1.5 + 0.3)) * 0.6 + Math.abs(Math.sin(t * 4.7 + 0.2)) * 0.3;
        peakIn = Math.max(rawL, rawR);
      }
      // EMA smoothing on level
      smL = smL + (rawL - smL) * LEVEL_EMA;
      smR = smR + (rawR - smR) * LEVEL_EMA;
      // Peak hold + decay
      pL = Math.max(peakIn, pL - 0.004);
      pR = Math.max(clamp(peakIn * 0.94), pR - 0.004);
      lvlL = smL;
      lvlR = smR;
      peakL = pL;
      peakR = pR;
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(frame);
    };
  });
</script>

<div class="vu-stereo" style="width: {width}px" role="img" aria-label="Stereo output levels">
  <div
    class="vu-row"
    style="--lvl: {clamp(lvlL) * 100}%; --peak: {clamp(peakL) * 100}%"
    aria-hidden="true"
  ></div>
  <div
    class="vu-row"
    style="--lvl: {clamp(lvlR) * 100}%; --peak: {clamp(peakR) * 100}%"
    aria-hidden="true"
  ></div>
</div>

<style>
  .vu-stereo {
    display: grid;
    grid-template-rows: 1fr 1fr;
    gap: 2px;
    height: 24px;
    flex-shrink: 0;
  }
  .vu-row {
    position: relative;
    background: var(--color-bg-3);
    border-radius: var(--radius-xs);
    overflow: hidden;
    border: 1px solid var(--color-line-2);
  }
  .vu-row::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: var(--lvl, 50%);
    background: linear-gradient(
      90deg,
      var(--color-primary) 0%,
      var(--color-primary) 60%,
      var(--color-signal-amber) 80%,
      var(--color-signal-orange) 95%
    );
  }
  .vu-row::after {
    content: '';
    position: absolute;
    left: calc(var(--peak, 80%) - 1px);
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--color-text-0);
    opacity: 0.85;
  }
</style>
