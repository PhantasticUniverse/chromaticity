<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-39 Meter widget — LED-segment-ladder peak meter ported from
  // the bundle's `components.jsx` lines 163-180. Single-channel
  // discrete-segment display, distinct from the W23 PR2 VuStereo
  // (continuous gradient stripe). The Settings → Audio → Monitoring
  // row consumes this with frozen-frame props per the bundle's
  // `<Meter level={0.68} peak={0.82} bars={28} />`; the Phase-3+/4a
  // Audio view will drive `level` + `peak` from the live cpal RMS /
  // peak-hold pair.
  //
  // Segment classification mirrors the bundle exactly: `on` for any
  // segment below the level cursor, `peak` for the held-peak marker
  // (which never falls behind the level), and `clip` for any "on"
  // segment in the top decile (`i / bars > 0.9`). The clip threshold
  // matches the bundle's amber→orange gradient on the VU meter — both
  // primitives mark the same dB region for dynamic-range cues.

  let {
    level = 0.6,
    bars = 20,
    peak,
    ariaLabel,
  }: {
    level?: number;
    bars?: number;
    peak?: number;
    ariaLabel?: string;
  } = $props();

  function clamp01(v: number): number {
    return Math.max(0, Math.min(1, v));
  }

  let lvl = $derived(clamp01(level));
  let held = $derived(Math.max(lvl, typeof peak === 'number' ? clamp01(peak) : lvl));
  let levelIdx = $derived(Math.floor(lvl * bars));
  let peakIdx = $derived(Math.max(levelIdx, Math.floor(held * bars)));
  let segments = $derived(
    Array.from({ length: bars }, (_, i) => {
      const on = i < levelIdx;
      const isPeak = i === peakIdx && peakIdx >= levelIdx;
      const clip = on && i / bars > 0.9;
      return { clip, isPeak, on };
    }),
  );
  let label = $derived(ariaLabel ?? `Level ${Math.round(lvl * 100)}%`);
</script>

<div class="meter" role="img" aria-label={label}>
  {#each segments as seg, i (i)}
    <div
      class="meter-bar"
      class:on={seg.on && !seg.clip && !seg.isPeak}
      class:peak={seg.isPeak && !seg.clip}
      class:clip={seg.clip}
    ></div>
  {/each}
</div>

<style>
  .meter {
    display: flex;
    gap: 2px;
    align-items: stretch;
    height: 14px;
    flex: 1;
    min-width: 0;
  }
  .meter-bar {
    flex: 1;
    min-width: 2px;
    background: var(--color-bg-3);
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-2);
    transition: background 0.08s ease;
  }
  .meter-bar.on {
    background: var(--color-primary);
    border-color: var(--color-primary);
  }
  .meter-bar.peak {
    background: var(--color-warning);
    border-color: var(--color-warning);
  }
  .meter-bar.clip {
    background: var(--color-danger);
    border-color: var(--color-danger);
  }
</style>
