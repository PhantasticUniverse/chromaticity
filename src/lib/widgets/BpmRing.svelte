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
  // already used by `CueTransitionCurve`'s ghost-dot animation. Frozen
  // frame contract: synthetic BPM only — no IPC, no audio-pipeline data.
  //
  // `prefers-reduced-motion: reduce` short-circuits the rAF tick and
  // pins the visual at beat 1 / measure 0 so the operator sees a static
  // frame instead of silent animation.

  let { bpm = 128, size = 96 }: { bpm?: number; size?: number } = $props();

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

  const beatPos = $derived((now * bpm) / 60);
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
  aria-label="BPM ring at {bpm} BPM, beat {beatIndex} of 4"
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
  <div class="bpm-ring__val">{bpm}</div>
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
