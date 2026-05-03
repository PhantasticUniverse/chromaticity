<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-23 PR1 widget primitive: BPM ring. A circular SVG progress ring
  // showing position within a 4-count measure at the given BPM. The
  // background ring is a thin neutral track; the foreground arc sweeps
  // from 0% to 100% over each measure (4 beats), then resets.
  //
  // Source of truth:
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
  // lines 343-358 (`const BpmRing = ({ bpm = 128 }) => …`). The bundle's
  // `useTick` rAF hook becomes a `$effect` here per the Svelte 5 idiom
  // already used by `CueTransitionCurve`'s ghost-dot animation.
  //
  // **Live IPC OR synthetic fallback (Wave 6 PR 2).** Per
  // `docs/adr/ADR-019-audio-uniform-ipc-bridge.md` §Rollout PR 2 this
  // widget consumes the engine's `audio_uniform_snapshot_changed` Tauri
  // event via `audioSnapshotStore` (producer side: Wave 6 PR 1, merge
  // `c1d93e8`). When a non-null snapshot is available the displayed BPM
  // is sourced from `liveSnapshot.bpm`; otherwise the synthetic
  // rAF-driven sweep keeps running so the ring still animates while the
  // production audio bootstrap is pending. Either way `prefers-
  // reduced-motion: reduce` pins the visual at beat 1 / measure 0.
  //
  // The optional `snapshot` prop lets tests inject a fixed payload
  // without going through the singleton store; default behaviour reads
  // from the store. `bpm` remains the visual default when neither path
  // delivers a value.

  import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';
  import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';

  let {
    bpm = 128,
    size = 96,
    snapshot = undefined,
  }: {
    bpm?: number;
    size?: number;
    snapshot?: AudioUniformSnapshotPayload | null;
  } = $props();

  // Mount-time subscribe so production widgets attach to the IPC stream
  // automatically; idempotent so multiple mounted visualizers share one
  // listener.
  $effect(() => {
    void audioSnapshotStore.subscribe();
  });

  const liveSnapshot = $derived(snapshot ?? audioSnapshotStore.snapshot);
  const liveBpm = $derived(liveSnapshot?.bpm ?? bpm);

  let now = $state(0);
  let reduceMotion = $state(false);

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
    if (reduceMotion) {
      now = 0;
      return;
    }
    let raf = 0;
    const start = performance.now();
    const tick = (t: number): void => {
      now = (t - start) / 1000;
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  const beatPos = $derived((now * liveBpm) / 60);
  const measureFrac = $derived((((beatPos / 4) % 1) + 1) % 1);
  const beatIndex = $derived(Math.floor(beatPos % 4) + 1);

  const VIEW = 40;
  const RADIUS = 16;
  const CIRC = 2 * Math.PI * RADIUS;
  const offset = $derived(CIRC * (1 - measureFrac));
</script>

<div
  class="bpm-ring"
  style="width: {size}px; height: {size}px;"
  role="img"
  aria-label="BPM ring at {liveBpm} BPM, beat {beatIndex} of 4"
>
  <svg viewBox="0 0 {VIEW} {VIEW}" aria-hidden="true">
    <circle class="bpm-ring__bg" cx={VIEW / 2} cy={VIEW / 2} r={RADIUS} />
    <circle
      class="bpm-ring__fg"
      cx={VIEW / 2}
      cy={VIEW / 2}
      r={RADIUS}
      stroke-dasharray={CIRC}
      stroke-dashoffset={offset}
    />
  </svg>
  <div class="bpm-ring__val">{liveBpm}</div>
</div>

<style>
  .bpm-ring {
    position: relative;
    display: inline-block;
  }
  .bpm-ring svg {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }
  .bpm-ring__bg {
    fill: none;
    stroke: var(--color-line-2);
    stroke-width: 3;
  }
  .bpm-ring__fg {
    fill: none;
    stroke: var(--color-primary);
    stroke-width: 3;
    stroke-linecap: round;
    transition: stroke-dashoffset 100ms linear;
    filter: drop-shadow(0 0 calc(4px * var(--glow, 1)) var(--color-primary));
  }
  .bpm-ring__val {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-md);
    font-weight: var(--font-weight-label);
    color: var(--color-text-0);
  }
</style>
