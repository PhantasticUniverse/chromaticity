<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import type { PointerEventHandler } from 'svelte/elements';

  // Wave-8 visual primitive: rotary knob. Wave 2.1 Lane A.2 wires
  // pointer-drag scrubbing + an `onchange` callback so the Perform
  // view's Scene Controls + Meta Controls panels can dispatch
  // `set_control` IPC as the user scrubs. Indicator sweeps -135° to
  // +135° (270° total) — matching the W8 visual contract.
  //
  // Drag convention: pointerdown captures the start clientY + the
  // value at press, pointermove maps Δy to Δvalue at SCRUB_PIXELS px
  // per full sweep (clamped to [0, 1]). Inverts Y so upward drag
  // increases the value (the natural rotary convention). Mirrors the
  // W23 PR2 CrossFader pointer-capture lifecycle (setPointerCapture →
  // pointermove → pointerup) with `touch-action: none`. Calls without
  // `onchange` and callers passing `disabled=true` short-circuit
  // before any dispatch.
  let {
    value,
    label,
    showValue = false,
    onchange,
    disabled = false,
  }: {
    value: number;
    label?: string;
    showValue?: boolean;
    onchange?: (next: number) => void;
    disabled?: boolean;
  } = $props();

  /// Pixels of vertical drag for a full 0 → 1 sweep. 150 px matches
  /// the bundle's reference DAW knob feel — fine enough for precise
  /// adjustment, coarse enough that a short drag spans the range.
  const SCRUB_PIXELS = 150;

  let dialEl = $state<HTMLDivElement | null>(null);
  let dragging = $state(false);
  let dragStartY = $state(0);
  let dragStartValue = $state(0);

  const pct = $derived(Math.max(0, Math.min(1, value)));
  const rot = $derived(-135 + pct * 270);

  // Render gate: presence of `onchange` switches to the role="slider"
  // surface; action gate: `disabled` separately suppresses scrub.
  // Mirrors `Fader.svelte` + `CrossFader.svelte`.
  const isSlider = $derived(onchange !== undefined);
  const actionable = $derived(!disabled && onchange !== undefined);

  const onPointerDown: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!actionable) return;
    event.preventDefault();
    dragging = true;
    dragStartY = event.clientY;
    dragStartValue = pct;
    const target = event.currentTarget;
    target.setPointerCapture(event.pointerId);
  };

  const onPointerMove: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!dragging || !actionable) return;
    const delta = (dragStartY - event.clientY) / SCRUB_PIXELS;
    const next = Math.max(0, Math.min(1, dragStartValue + delta));
    onchange?.(next);
  };

  const onPointerUp: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!dragging) return;
    dragging = false;
    const target = event.currentTarget;
    if (target.hasPointerCapture(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }
  };

  // Keyboard nudge per WAI-ARIA APG slider pattern. Rotary knobs accept
  // ↑/→ as "clockwise / increase" and ↓/← as "counter-clockwise / decrease"
  // by convention. Step sizing mirrors `Fader.svelte` for cross-widget
  // operator muscle memory: 5% per arrow, 20% with Shift / PageUp / PageDown,
  // Home/End snap to extremes. Closes the DESIGN.md "Bundle alignment"
  // §Trail item ("Arrow-key slider nudge").
  const STEP_SMALL = 0.05;
  const STEP_LARGE = 0.2;
  const onKeyDown = (event: KeyboardEvent) => {
    if (!actionable) return;
    let next: number | null = null;
    switch (event.key) {
      case 'ArrowUp':
      case 'ArrowRight':
        next = pct + (event.shiftKey ? STEP_LARGE : STEP_SMALL);
        break;
      case 'ArrowDown':
      case 'ArrowLeft':
        next = pct - (event.shiftKey ? STEP_LARGE : STEP_SMALL);
        break;
      case 'PageUp':
        next = pct + STEP_LARGE;
        break;
      case 'PageDown':
        next = pct - STEP_LARGE;
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = 1;
        break;
    }
    if (next !== null) {
      event.preventDefault();
      onchange?.(Math.max(0, Math.min(1, next)));
    }
  };
</script>

<!-- Interactive (role="slider" + tabindex + aria-value*) vs display-
     only (role="presentation") branches are duplicated so vite-
     plugin-svelte's a11y static analysis can verify each
     independently — the previous ternary form tripped
     `a11y_no_noninteractive_tabindex`. -->
<div class="knob" class:knob--disabled={disabled}>
  {#if isSlider}
    <div
      bind:this={dialEl}
      class="knob-dial"
      role="slider"
      tabindex="0"
      aria-valuemin={0}
      aria-valuemax={1}
      aria-valuenow={pct}
      aria-label={label ?? 'knob'}
      aria-disabled={disabled ? 'true' : undefined}
      style="--pct: {pct}; --rot: {rot}deg"
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
      onkeydown={onKeyDown}
    >
      <div class="knob-arc"></div>
      <div class="knob-ind"></div>
    </div>
  {:else}
    <div
      bind:this={dialEl}
      class="knob-dial"
      role="presentation"
      style="--pct: {pct}; --rot: {rot}deg"
    >
      <div class="knob-arc"></div>
      <div class="knob-ind"></div>
    </div>
  {/if}
  {#if showValue}<div class="knob-val">{value.toFixed(2)}</div>{/if}
  {#if label}<div class="knob-label">{label}</div>{/if}
</div>

<style>
  .knob {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-xs);
    min-width: 46px;
  }
  .knob--disabled {
    opacity: 0.5;
    pointer-events: none;
  }
  .knob-dial {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    position: relative;
    background: radial-gradient(
      circle at 30% 25%,
      var(--color-bg-5),
      var(--color-bg-3) 55%,
      var(--color-bg-1)
    );
    border: 1px solid var(--color-line-3);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.05),
      inset 0 -2px 4px rgba(0, 0, 0, 0.5);
    touch-action: none;
    user-select: none;
  }
  .knob-dial[role='slider'] {
    cursor: ns-resize;
  }
  .knob-dial:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .knob-arc {
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    background: conic-gradient(
      from 225deg,
      var(--tint, var(--color-primary)) calc(var(--pct, 0.5) * 270deg),
      transparent 0
    );
    mask: radial-gradient(circle, transparent 52%, #000 54%, #000 60%, transparent 62%);
    -webkit-mask: radial-gradient(circle, transparent 52%, #000 54%, #000 60%, transparent 62%);
    filter: drop-shadow(0 0 calc(4px * var(--glow)) var(--tint, var(--color-primary)));
    pointer-events: none;
  }
  .knob-ind {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 2px;
    height: calc(48px * 0.4);
    background: linear-gradient(180deg, var(--tint, var(--color-primary)), var(--color-text-0));
    border-radius: 2px;
    transform-origin: 50% 100%;
    transform: translate(-50%, -100%) rotate(var(--rot, 0deg));
    box-shadow: 0 0 calc(6px * var(--glow)) var(--tint, var(--color-primary));
    pointer-events: none;
  }
  .knob-val {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
    line-height: 1;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .knob-label {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-caps-md);
    font-weight: var(--font-weight-label);
    color: var(--color-text-1);
    text-align: center;
    line-height: 1.15;
  }
</style>
