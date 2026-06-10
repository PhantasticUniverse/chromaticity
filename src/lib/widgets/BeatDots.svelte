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
  // the beat-position math is sourced from `liveSnapshot.bpm` only
  // once `bpm_availability === 'Ready'`; during warmup the synthetic
  // BPM keeps cycling and the accessible label reports warming.
  // `prefers-reduced-motion: reduce` pins beat 1 (the downbeat)
  // regardless.
  //
  // The optional `snapshot` prop lets tests inject a fixed payload
  // without going through the singleton store; default behaviour reads
  // from the store.
  //
  // **Incremental beat phase.** The beat position is integrated per
  // rAF frame (`phase += dt · bpm / 60`, wrapped to [0, 4) beats)
  // instead of recomputed as elapsed-time × instantaneous-BPM, so a
  // live BPM estimator update changes only the future stepping rate
  // and never teleports the active dot (the old absolute product
  // jumped by `elapsed · ΔBPM / 60` beats on every estimator wobble).

  import { untrack } from 'svelte';

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
  const bpmReady = $derived(liveSnapshot === null || liveSnapshot.bpm_availability === 'Ready');
  const liveBpm = $derived(liveSnapshot && bpmReady ? liveSnapshot.bpm : bpm);
  // The wire BPM is a fractional f32 (60·sr/lag, median-smoothed);
  // round to the integer the bundle shows for the accessible label.
  const bpmRounded = $derived(Math.round(liveBpm));

  // Beat-phase accumulator in beats, wrapped to [0, 4).
  let phase = $state(0);
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
      phase = 0;
      return;
    }
    let raf = 0;
    let last: number | null = null;
    const tick = (t: number): void => {
      const dt = last === null ? 0 : Math.max(0, (t - last) / 1000);
      last = t;
      // rAF callbacks run outside the tracked context; `untrack` makes
      // the per-frame BPM read explicitly dependency-free so an engine
      // BPM tick never restarts this effect (which would re-anchor the
      // clock).
      phase = (phase + (dt * untrack(() => liveBpm)) / 60) % 4;
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  });

  const cur = $derived(Math.floor(phase));

  const DOTS: ReadonlyArray<number> = [0, 1, 2, 3];
</script>

<div
  class="beat-dots"
  role="img"
  aria-label={liveSnapshot !== null && !bpmReady
    ? `Beat ${cur + 1} of 4, BPM warming`
    : `Beat ${cur + 1} of 4 at ${bpmRounded} BPM`}
>
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
