<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave 2.1 Lane B.2: widened with an optional `snapshot` prop so the
  // top-bar HUD mount can drive levels from the live audio uniform
  // store. The payload's `rms` is mono — we mirror it to both L and R
  // (with a tiny offset for visual separation) until a stereo
  // uniform-snapshot extension exists. Without a snapshot the widget
  // falls back to its prior synthetic rAF generator, preserving every
  // existing call-site's behaviour.
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

    // Live-snapshot dual-mode: when the audio uniform store has emitted
    // at least one snapshot, drive levels from `rms` / `peak`. The
    // payload's `rms` is mono per ADR-019; we mirror with a small
    // offset so the two rows aren't visually identical.
    if (liveSnapshot) {
      const rms = clamp(liveSnapshot.rms);
      const pk = clamp(liveSnapshot.peak);
      lvlL = rms;
      lvlR = clamp(rms * 0.94);
      peakL = pk;
      peakR = clamp(pk * 0.94);
      return;
    }

    let frame = 0;
    const start = performance.now();
    let pL = 0;
    let pR = 0;

    const tick = (now: number) => {
      const t = (now - start) / 1000;
      const l = Math.abs(Math.sin(t * 1.7)) * 0.6 + Math.abs(Math.sin(t * 5)) * 0.3;
      const r = Math.abs(Math.sin(t * 1.5 + 0.3)) * 0.6 + Math.abs(Math.sin(t * 4.7 + 0.2)) * 0.3;
      pL = Math.max(l, pL - 0.004);
      pR = Math.max(r, pR - 0.004);
      lvlL = l;
      lvlR = r;
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
