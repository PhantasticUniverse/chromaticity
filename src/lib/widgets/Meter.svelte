<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // World-class LED-segment level meter. Ported in shape from the
  // bundle's `components.jsx` segment ladder, but rebuilt onto a
  // LOGARITHMIC dBFS scale (see `src/lib/audio/metering.ts`): segments
  // map to dB positions, not a linear [0,1] fill, so a healthy but quiet
  // program (-20 dBFS) lights ~2/3 of the bar instead of the bottom
  // tenth. Each segment is colored by the dB IT represents (green
  // working zone, amber caution, red hot), and a floating peak-hold cap
  // tracks the recent maximum.
  //
  // Inputs are dBFS values already smoothed by the shared metering
  // ballistics in the CALLER (the caller owns the dt-aware envelope so
  // the same primitive serves every meter). `silent` renders the
  // reference's whole-bar no-signal state.
  //
  // SCOPE: the red zone here is amplitude-hot (>= -1 dBFS), an honest
  // "very loud" cue — TRUE clip/over detection (samples >= 0 dBFS,
  // inter-sample true-peak) needs the engine-side widening that stops
  // clamping the level path, tracked separately (ADR-019 amendment).

  import {
    dbToFraction,
    formatDb,
    fractionToDb,
    METER_FLOOR_DB,
    REFERENCE_TICK_DB,
    zoneForDb,
    type MeterZone,
  } from '../audio/metering';

  let {
    levelDb = METER_FLOOR_DB,
    peakDb,
    bars = 20,
    silent = false,
    ariaLabel,
    showReadout = false,
    onResetPeak,
  }: {
    /** Current smoothed level in dBFS (already ballistics-shaped by the caller). */
    levelDb?: number;
    /** Held peak marker in dBFS. Omitted => no cap (tracks the level). */
    peakDb?: number;
    bars?: number;
    /** Render the no-signal state (reference's whole-bar-red equivalent). */
    silent?: boolean;
    ariaLabel?: string;
    /** Show the numeric current/held dB readout beside the bar. */
    showReadout?: boolean;
    /** When provided, a reset affordance clears the held peak. */
    onResetPeak?: () => void;
  } = $props();

  const heldDb = $derived(typeof peakDb === 'number' ? Math.max(peakDb, levelDb) : levelDb);
  const levelIdx = $derived(Math.round(dbToFraction(levelDb) * bars));
  const peakIdx = $derived(
    Math.min(bars - 1, Math.max(levelIdx, Math.round(dbToFraction(heldDb) * bars))),
  );
  // Segment nearest the EBU -18 dBFS alignment tick.
  const refIdx = $derived(Math.round(dbToFraction(REFERENCE_TICK_DB) * bars));

  const segments = $derived(
    Array.from({ length: bars }, (_, i) => {
      // dB at the top edge of this segment colors it by zone.
      const segDb = fractionToDb((i + 1) / bars);
      const on = i < levelIdx;
      const isPeak = i === peakIdx && peakIdx >= levelIdx;
      const zone: MeterZone = zoneForDb(segDb);
      const isRef = i === refIdx;
      return { on, isPeak, zone, isRef };
    }),
  );

  // Always surface the no-signal state to assistive tech, even when a
  // caller passes an explicit ariaLabel (otherwise the visible
  // "no signal" cue is invisible behind role="img" + the override).
  const label = $derived(
    silent
      ? `${ariaLabel ?? 'Level'} — no signal`
      : (ariaLabel ?? `Level ${formatDb(levelDb)} dBFS`),
  );
</script>

<div class="meter-row">
  <div class="meter" class:silent role="img" aria-label={label}>
    {#each segments as seg, i (i)}
      <div
        class="meter-bar"
        class:on={seg.on}
        class:green={seg.on && seg.zone === 'green'}
        class:amber={seg.on && seg.zone === 'amber'}
        class:orange={seg.on && seg.zone === 'orange'}
        class:red={seg.on && seg.zone === 'red'}
        class:peak={seg.isPeak}
        class:ref={seg.isRef}
      ></div>
    {/each}
    {#if silent}
      <span class="meter__no-signal">no signal</span>
    {/if}
  </div>
  {#if showReadout}
    <div class="meter__readout">
      <span class="meter__db" class:silent>{silent ? '—' : formatDb(levelDb)}</span>
      <span class="meter__db-unit">dBFS</span>
      <span class="meter__held" title="held peak">⌃{formatDb(heldDb)}</span>
      {#if onResetPeak}
        <button type="button" class="meter__reset" title="Reset peak hold" onclick={onResetPeak}>
          ⟲
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .meter-row {
    display: flex;
    align-items: center;
    gap: var(--space-2, 8px);
    flex: 1;
    min-width: 0;
  }
  .meter {
    position: relative;
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
    /* FIX 2 — no CSS transition on the segment fill. The DSP envelope in
       the caller is the only smoothing layer; tweening the bar color over
       a fixed duration just adds that duration as fixed lag on a moving
       target and makes LED segments smear instead of snapping on/off. */
  }
  /* dB-zone fill colors (per-segment by the dB it represents). */
  .meter-bar.green {
    background: var(--color-ok, var(--color-primary));
    border-color: var(--color-ok, var(--color-primary));
  }
  .meter-bar.amber {
    background: var(--color-warning);
    border-color: var(--color-warning);
  }
  .meter-bar.orange {
    background: var(--color-signal-orange);
    border-color: var(--color-signal-orange);
  }
  .meter-bar.red {
    background: var(--color-danger);
    border-color: var(--color-danger);
  }
  /* Floating peak-hold cap, drawn over whatever fill is beneath it. */
  .meter-bar.peak {
    background: var(--color-text-1, #fff);
    border-color: var(--color-text-1, #fff);
  }
  /* EBU -18 dBFS alignment tick: a subtle top notch. */
  .meter-bar.ref {
    box-shadow: inset 0 2px 0 var(--color-line-3, rgba(255, 255, 255, 0.35));
  }
  /* No-signal: dim the bar and tint its frame, mirroring the reference's
     whole-bar no-signal cue (our chain pill carries the richer state). */
  .meter.silent .meter-bar {
    background: var(--color-bg-3);
    border-color: var(--color-danger);
    opacity: 0.5;
  }
  .meter__no-signal {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-danger);
    pointer-events: none;
  }
  .meter__readout {
    display: flex;
    align-items: baseline;
    gap: 4px;
    font-variant-numeric: tabular-nums;
    font-size: 11px;
    white-space: nowrap;
  }
  .meter__db {
    color: var(--color-text-1);
    min-width: 3.5ch;
    text-align: right;
  }
  .meter__db.silent {
    color: var(--color-text-3);
  }
  .meter__db-unit,
  .meter__held {
    color: var(--color-text-3);
    font-size: 10px;
  }
  .meter__reset {
    background: none;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    color: var(--color-text-3);
    cursor: pointer;
    padding: 0 4px;
    line-height: 1.4;
    font-size: 10px;
  }
  .meter__reset:hover {
    color: var(--color-text-1);
    border-color: var(--color-line-1, var(--color-line-2));
  }
</style>
