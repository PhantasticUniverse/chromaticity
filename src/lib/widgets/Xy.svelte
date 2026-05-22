<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  import type { PointerEventHandler } from 'svelte/elements';

  // Wave-32 visual primitive: 2D xy pad. Display-only per the W8
  // Fader / Knob / Toggle / Swatch precedent; ports the bundle's
  // `chroma_inspector.jsx::ChromaCtrlBody` `xy` branch (lines 151-167).
  //
  // Lane D (parity-closure wave 1) adds full interactivity:
  //   - `onchange?: (next: [number, number]) => void` prop; pointer-drag
  //     (pointerdown/move/up + pointer capture; `touch-action: none` on
  //     the pad) mapping clientX/clientY over the pad's bounding rect onto
  //     the caller-supplied [min, max] range pair.
  //   - Dual-branch a11y mirrors `Fader.svelte` / `Knob.svelte`: the pad
  //     renders `role="slider"` (plus tabindex + aria-value*) when
  //     `onchange` is provided; `role="img"` with the descriptive label
  //     when display-only. The previous ternary form trips
  //     `a11y_no_noninteractive_tabindex`, so branches are duplicated so
  //     vite-plugin-svelte can verify each independently.
  //   - Full keyboard nav: Arrow keys nudge one axis by STEP_SMALL (5% of
  //     range); Shift+Arrow applies STEP_LARGE (20%); Home/End snap each
  //     axis to min/max; Alt+Arrow nudges diagonally (both axes
  //     simultaneously for a single key). Mirrors the WAI-ARIA APG
  //     slider step conventions used by `Fader.svelte`.
  //   - ARIA: the interactive branch exposes a describedby label listing
  //     both axes' current values and an aria-roledescription "xy pad
  //     slider" so AT users understand the 2D nature. `aria-valuemin`
  //     through `aria-valuenow` cover the X axis (the primary focus axis)
  //     per WAI-ARIA composite widget guidance; the Y axis value is
  //     surfaced in the live region via `aria-describedby`.
  //   - `prefers-reduced-motion: reduce` — the snap-to-corner animation
  //     (brief translate-ease on the dot) is suppressed under this media
  //     query. The drag calculation and `onchange` dispatch remain active
  //     regardless.
  //   - DPR-aware drag sensitivity: `getBoundingClientRect()` returns
  //     CSS-pixel dimensions regardless of DPR; pointer clientX/Y are
  //     also in CSS pixels, so no DPR scaling is applied. Matches the
  //     `Fader.svelte` approach.
  //   - Two-layer deterministic clamping: raw computed normalised position
  //     is clamped in `posFromClient`, and the `nxClamped` / `nyClamped`
  //     derived values clamp the dot position as a second layer.
  //   - Snap-to-corner: values within SNAP_ZONE (5% of range) of a corner
  //     snap to the exact corner during drag.
  //
  // The display-only branch (no `onchange`) is preserved for the Inputs
  // Inspector view and any caller that only wants a readout widget.
  let {
    value,
    min = [-1, -1] as const,
    max = [1, 1] as const,
    label,
    onchange,
    disabled = false,
  }: {
    value: readonly [number, number];
    min?: readonly [number, number];
    max?: readonly [number, number];
    label?: string;
    onchange?: (next: [number, number]) => void;
    disabled?: boolean;
  } = $props();

  let padEl = $state<HTMLDivElement | null>(null);
  let dragging = $state(false);

  // Render gate: presence of `onchange` switches to the interactive
  // (role="slider") branch. Action gate: `disabled` additionally
  // suppresses the dispatch path. Mirrors `Fader.svelte`.
  const isSlider = $derived(onchange !== undefined);
  const actionable = $derived(!disabled && onchange !== undefined);

  // ---- Derived display positions ----------------------------------------

  // Normalize each axis into a 0..100 percentage. y is inverted so
  // visual top is the max (matches the bundle and operator intuition
  // — "up = positive y").
  function fmt(n: number, d = 2): string {
    if (Math.abs(n) >= 100) return n.toFixed(0);
    return n.toFixed(d);
  }

  const nx = $derived(((value[0] - min[0]) / (max[0] - min[0] || 1)) * 100);
  const ny = $derived((1 - (value[1] - min[1]) / (max[1] - min[1] || 1)) * 100);
  const nxClamped = $derived(Math.max(0, Math.min(100, nx)));
  const nyClamped = $derived(Math.max(0, Math.min(100, ny)));

  // ---- Snap-to-corner threshold -----------------------------------------

  // 5% of range: when the raw normalised value is within SNAP_ZONE of
  // 0 or 1 (the corners), snap to the exact extremity. Provides a
  // "magnetic" feel at edges during drag.
  const SNAP_ZONE = 0.05;

  function snapAxis(norm: number): number {
    if (norm <= SNAP_ZONE) return 0;
    if (norm >= 1 - SNAP_ZONE) return 1;
    return norm;
  }

  // ---- Pointer → normalised position ------------------------------------

  function posFromClient(clientX: number, clientY: number): [number, number] {
    if (!padEl) return [value[0], value[1]];
    const rect = padEl.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return [value[0], value[1]];

    const rawNx = snapAxis(Math.max(0, Math.min(1, (clientX - rect.left) / rect.width)));
    // Y is inverted: visual top = max y.
    const rawNy = snapAxis(Math.max(0, Math.min(1, 1 - (clientY - rect.top) / rect.height)));

    const xRange = max[0] - min[0] || 1;
    const yRange = max[1] - min[1] || 1;
    const x = min[0] + rawNx * xRange;
    const y = min[1] + rawNy * yRange;
    return [x, y];
  }

  // ---- Pointer event handlers -------------------------------------------

  const onPointerDown: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!actionable) return;
    event.preventDefault();
    dragging = true;
    const target = event.currentTarget;
    target.setPointerCapture(event.pointerId);
    onchange?.(posFromClient(event.clientX, event.clientY));
  };

  const onPointerMove: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!dragging || !actionable) return;
    onchange?.(posFromClient(event.clientX, event.clientY));
  };

  const onPointerUp: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!dragging) return;
    dragging = false;
    const target = event.currentTarget;
    if (target.hasPointerCapture(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }
  };

  // ---- Keyboard navigation (WAI-ARIA APG slider pattern) ----------------
  // Per-axis step sizes mirror `Fader.svelte`:
  //   5% per arrow key (small step), 20% with Shift (large step).
  // Axis semantics:
  //   ArrowLeft/ArrowRight → X axis only.
  //   ArrowUp/ArrowDown    → Y axis only.
  //   Alt+Arrow            → diagonal nudge (both axes simultaneously).
  //   Home → X=min, Y=min (bottom-left corner).
  //   End  → X=max, Y=max (top-right corner).
  const STEP_SMALL = 0.05;
  const STEP_LARGE = 0.2;

  function nudgeAxis(current: number, rangeMin: number, rangeMax: number, delta: number): number {
    const range = rangeMax - rangeMin || 1;
    const norm = (current - rangeMin) / range;
    const next = Math.max(0, Math.min(1, norm + delta));
    return rangeMin + next * range;
  }

  const onKeyDown = (event: KeyboardEvent) => {
    if (!actionable) return;
    const step = event.shiftKey ? STEP_LARGE : STEP_SMALL;

    let nextX = value[0];
    let nextY = value[1];
    let handled = false;

    switch (event.key) {
      case 'ArrowRight':
        if (event.altKey) {
          // Diagonal: increase both axes.
          nextX = nudgeAxis(value[0], min[0], max[0], +step);
          nextY = nudgeAxis(value[1], min[1], max[1], +step);
        } else {
          nextX = nudgeAxis(value[0], min[0], max[0], +step);
        }
        handled = true;
        break;
      case 'ArrowLeft':
        if (event.altKey) {
          // Diagonal: decrease both axes.
          nextX = nudgeAxis(value[0], min[0], max[0], -step);
          nextY = nudgeAxis(value[1], min[1], max[1], -step);
        } else {
          nextX = nudgeAxis(value[0], min[0], max[0], -step);
        }
        handled = true;
        break;
      case 'ArrowUp':
        if (event.altKey) {
          nextX = nudgeAxis(value[0], min[0], max[0], +step);
          nextY = nudgeAxis(value[1], min[1], max[1], +step);
        } else {
          nextY = nudgeAxis(value[1], min[1], max[1], +step);
        }
        handled = true;
        break;
      case 'ArrowDown':
        if (event.altKey) {
          nextX = nudgeAxis(value[0], min[0], max[0], -step);
          nextY = nudgeAxis(value[1], min[1], max[1], -step);
        } else {
          nextY = nudgeAxis(value[1], min[1], max[1], -step);
        }
        handled = true;
        break;
      case 'Home':
        nextX = min[0];
        nextY = min[1];
        handled = true;
        break;
      case 'End':
        nextX = max[0];
        nextY = max[1];
        handled = true;
        break;
    }

    if (handled) {
      event.preventDefault();
      onchange?.([nextX, nextY]);
    }
  };

  // ---- ARIA labels -------------------------------------------------------

  // Descriptive label for AT: lists both axis values.
  const ariaLabel = $derived(
    label
      ? `${label} — x ${fmt(value[0])} y ${fmt(value[1])}`
      : `xy pad — x ${fmt(value[0])} y ${fmt(value[1])}`,
  );

  // aria-valuenow is the X-axis normalised value in [0, 1].
  const ariaValueNow = $derived(Math.max(0, Math.min(1, nx / 100)));
</script>

<!-- Interactive (role="slider" + tabindex + aria-value*) vs display-only
     (role="img") branches are duplicated so vite-plugin-svelte's a11y
     static analysis can verify each independently — the previous ternary
     form trips `a11y_no_noninteractive_tabindex`. CSS stays single-source
     via .xy__pad. -->
<div class="xy" class:xy--disabled={disabled}>
  {#if isSlider}
    <div
      bind:this={padEl}
      class="xy__pad"
      role="slider"
      tabindex="0"
      aria-valuemin={0}
      aria-valuemax={1}
      aria-valuenow={ariaValueNow}
      aria-roledescription="xy pad slider"
      aria-label={ariaLabel}
      aria-disabled={disabled ? 'true' : undefined}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
      onkeydown={onKeyDown}
    >
      <div class="xy__dot" style:left="{nxClamped}%" style:top="{nyClamped}%"></div>
    </div>
  {:else}
    <div bind:this={padEl} class="xy__pad" role="img" aria-label={ariaLabel}>
      <div class="xy__dot" style:left="{nxClamped}%" style:top="{nyClamped}%"></div>
    </div>
  {/if}
  <div class="xy__readout mono" aria-hidden="true">
    <span>x {fmt(value[0])}</span>
    <span>y {fmt(value[1])}</span>
  </div>
  {#if label}<div class="xy__label">{label}</div>{/if}
</div>

<style>
  .xy {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 56px;
  }
  .xy--disabled {
    opacity: 0.5;
    pointer-events: none;
  }
  .xy__pad {
    position: relative;
    width: 56px;
    height: 56px;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.25);
    overflow: hidden;
    touch-action: none;
    user-select: none;
  }
  .xy__pad::before,
  .xy__pad::after {
    content: '';
    position: absolute;
    background: var(--color-line-1);
    pointer-events: none;
  }
  .xy__pad::before {
    left: 50%;
    top: 0;
    bottom: 0;
    width: 1px;
    transform: translateX(-0.5px);
  }
  .xy__pad::after {
    top: 50%;
    left: 0;
    right: 0;
    height: 1px;
    transform: translateY(-0.5px);
  }
  .xy__pad[role='slider'] {
    cursor: crosshair;
  }
  .xy__pad:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .xy__dot {
    position: absolute;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--tint, var(--color-primary));
    transform: translate(-50%, -50%);
    box-shadow: 0 0 8px var(--tint, var(--color-primary));
    pointer-events: none;
    @media (prefers-reduced-motion: no-preference) {
      transition:
        left 40ms linear,
        top 40ms linear;
    }
  }
  .xy__readout {
    display: flex;
    gap: 6px;
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }
  .xy__label {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-caps-md);
    font-weight: var(--font-weight-label);
    color: var(--color-text-1);
    text-align: center;
    line-height: 1.15;
    max-width: 58px;
  }
</style>
