<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-23 PR1 widget primitive: 4-count beat dots. A row of four
  // dots representing beat positions within a measure. The dot whose
  // index matches the current beat at the given BPM is highlighted —
  // beat 1 (the downbeat) draws on `--color-cat-motion`, beats 2–4 on
  // `--color-primary`. Inactive dots stay muted on `--color-line-2`.
  //
  // Source of truth:
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/components.jsx`
  // lines 360-369 (`const BeatDots = ({ bpm = 128 }) => …`). The bundle
  // tints the downbeat (`i === 0`) with the orange accent token and the
  // remaining beats with the primary accent; here the downbeat reuses
  // the cyan motion-category token so the divergence between downbeat
  // and the off-beats is theme-token-driven.
  //
  // **Live IPC OR synthetic fallback (Wave 6 PR 2).** Per
  // `docs/adr/ADR-019-audio-uniform-ipc-bridge.md` §Rollout PR 2 this
  // widget consumes the engine's `audio_uniform_snapshot_changed` Tauri
  // event via `audioSnapshotStore` (producer side: Wave 6 PR 1, merge
  // `c1d93e8`). When a non-null snapshot is available the BPM driving
  // the beat-position math is sourced from `liveSnapshot.bpm`;
  // otherwise the synthetic rAF tick keeps cycling so the dots still
  // animate. `prefers-reduced-motion: reduce` pins beat 1 (the
  // downbeat) regardless.
  //
  // The optional `snapshot` prop lets tests inject a fixed payload
  // without going through the singleton store; default behaviour reads
  // from the store.

  import type { AudioUniformSnapshotPayload } from '../ipc/bindings/AudioUniformSnapshotPayload';
  import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';

  let {
    bpm = 128,
    snapshot = undefined,
  }: {
    bpm?: number;
    snapshot?: AudioUniformSnapshotPayload | null;
  } = $props();

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

  const cur = $derived(Math.floor(((now * liveBpm) / 60) % 4));

  const DOTS: ReadonlyArray<number> = [0, 1, 2, 3];
</script>

<div class="beat-dots" role="img" aria-label="Beat {cur + 1} of 4 at {liveBpm} BPM">
  {#each DOTS as i (i)}
    <span
      class="beat-dots__dot"
      class:beat-dots__dot--down={i === cur && i === 0}
      class:beat-dots__dot--on={i === cur && i !== 0}
      aria-hidden="true"
    ></span>
  {/each}
</div>

<style>
  .beat-dots {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .beat-dots__dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    transition:
      background-color 80ms linear,
      box-shadow 80ms linear,
      border-color 80ms linear;
  }
  .beat-dots__dot--on {
    background: var(--color-primary);
    border-color: var(--color-primary);
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--color-primary);
  }
  .beat-dots__dot--down {
    background: var(--color-cat-motion);
    border-color: var(--color-cat-motion);
    box-shadow: 0 0 calc(10px * var(--glow, 1)) var(--color-cat-motion);
  }
</style>
