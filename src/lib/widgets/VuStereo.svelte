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

  import { AttackReleaseEnvelope, PeakHold, ampToDb, dbToFraction } from '../audio/metering';
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

  // Frozen demo pose (live=false / reduced-motion): meter-travel FRACTIONS
  // (0..1), so 0.6 reads as a 60%-filled bar. The live path writes these
  // same [0,1] fractions via the dBFS map, so frozen and live share units.
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

    // DAW-grade, dt-aware ballistics on a logarithmic dBFS scale (see
    // docs/dev/ui-smoothness-and-latency.md). A fast-attack / slow-release
    // follower drives the gradient fill; a peak-hold marker latches the
    // loudest recent peak and glides down. Both are wall-clock-dt-aware
    // (`coeff = 1 - exp(-dt/tau)`), so the feel is identical on 60 / 120 /
    // 144 Hz displays — the prior per-frame EMA (`* 0.25`) + per-frame
    // decay (`- 0.004`) ran twice as fast on a 120 Hz panel. `dbToFraction`
    // replaces the old LINEAR `[0,1]` fill that crushed a healthy-but-quiet
    // program into the bottom tenth of the bar. rms/peak are mono today,
    // mirrored to L/R with a small -1 dB R offset for visual separation,
    // until a stereo uniform-snapshot extension lands (ADR-023 §2).
    const levelL = new AttackReleaseEnvelope();
    const levelR = new AttackReleaseEnvelope();
    const peakHoldL = new PeakHold();
    const peakHoldR = new PeakHold();

    let frame = 0;
    let lastTs = performance.now();
    const startTs = lastTs;

    const tick = (now: number): void => {
      const dtMs = Math.max(0, now - lastTs);
      lastTs = now;
      // Untrack the live-snapshot read so the $effect doesn't tear down +
      // recreate the rAF loop on every ~60 Hz engine snapshot write.
      const liveNow = untrack(() => liveSnapshot);
      let rmsAmp: number;
      let peakAmp: number;
      if (liveNow) {
        rmsAmp = liveNow.rms;
        peakAmp = liveNow.peak;
      } else {
        // Synthetic demo motion when no live IPC is present (bundle pose).
        const t = (now - startTs) / 1000;
        rmsAmp = Math.abs(Math.sin(t * 1.7)) * 0.6 + Math.abs(Math.sin(t * 5)) * 0.3;
        peakAmp = Math.min(0.995, rmsAmp + 0.1);
      }
      const rmsDb = ampToDb(rmsAmp);
      const peakDb = ampToDb(peakAmp);
      lvlL = dbToFraction(levelL.process(rmsDb, dtMs));
      lvlR = dbToFraction(levelR.process(rmsDb - 1, dtMs));
      peakL = dbToFraction(peakHoldL.process(peakDb, dtMs));
      peakR = dbToFraction(peakHoldR.process(peakDb - 1, dtMs));
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
    /* Stops anchored to dB zone boundaries on the -60..0 dBFS travel the
       fill width now maps to (dbToFraction): -12 dBFS = 80%, -6 = 90%,
       -3 = 95%, 0 = 100%. Matches metering.ts zoneForDb + Meter.svelte. */
    background: linear-gradient(
      90deg,
      var(--color-primary) 0%,
      var(--color-primary) 80%,
      var(--color-signal-amber) 88%,
      var(--color-signal-orange) 95%,
      var(--color-danger) 100%
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
