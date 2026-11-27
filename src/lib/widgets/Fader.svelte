<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import type { PointerEventHandler } from 'svelte/elements';

  // Wave-8 visual primitive: vertical fader. Wave 2.1 Lane A.2 wires
  // pointer drag + an `onchange` callback so the Perform view's Scene
  // Controls + Meta Controls panels can dispatch `set_control` IPC as
  // the user drags. The widget stays a controlled component — `value`
  // is the source of truth from the caller, and pointer-drag emits
  // `onchange(next)` for the caller to commit (or ignore, for display-
  // only callers like the Inputs Inspector view).
  //
  // Pointer-drag convention mirrors the W23 PR2 `CrossFader.svelte`
  // precedent — pointerdown → setPointerCapture → pointermove → pointerup
  // with `touch-action: none` on the track. clientY is normalized to
  // [0, 1] over the track's `getBoundingClientRect()` height (top = 1,
  // bottom = 0 — vertical-fader semantics). Calls without `onchange`
  // and callers passing `disabled=true` short-circuit before any
  // dispatch.
  //
  // The optional modulation halo is a pure-CSS @keyframes pulse on a
  // ::after pseudo-element so no JS tick loop is required.
  let {
    value,
    label,
    modulated = false,
    onchange,
    disabled = false,
  }: {
    value: number;
    label?: string;
    modulated?: boolean;
    onchange?: (next: number) => void;
    disabled?: boolean;
  } = $props();

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragging = $state(false);

  const pct = $derived(Math.max(0, Math.min(1, value)));

  function valueFromClientY(clientY: number): number {
    if (!trackEl) return value;
    const rect = trackEl.getBoundingClientRect();
    if (rect.height <= 0) return value;
    // Vertical fader: top of track = 1, bottom = 0.
    const raw = 1 - (clientY - rect.top) / rect.height;
    return Math.max(0, Math.min(1, raw));
  }

  // Render gate: presence of `onchange` switches to the role="slider"
  // surface so the widget reports correct accessibility semantics even
  // when temporarily disabled (the test surface + screen readers can
  // observe `aria-disabled` on a slider rather than seeing a different
  // element shape blink in / out). Action gate: `disabled` separately
  // suppresses the dispatch path. Mirrors the `CrossFader.svelte`
  // precedent.
  const isSlider = $derived(onchange !== undefined);
  const actionable = $derived(!disabled && onchange !== undefined);

  const onPointerDown: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!actionable) return;
    event.preventDefault();
    dragging = true;
    const target = event.currentTarget;
    target.setPointerCapture(event.pointerId);
    onchange?.(valueFromClientY(event.clientY));
  };

  const onPointerMove: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!dragging || !actionable) return;
    onchange?.(valueFromClientY(event.clientY));
  };

  const onPointerUp: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!dragging) return;
    dragging = false;
    const target = event.currentTarget;
    if (target.hasPointerCapture(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }
  };

  // Keyboard nudge per WAI-ARIA APG slider pattern + the v0.2-beta bundle's
  // operator habit (vertical fader responds to ↑/↓ as primary; ←/→ are
  // accepted aliases for the operator who's also holding a mouse). Step
  // sizing matches HTML5 input[type=range] convention scaled to the 0..1
  // range: 5% per arrow, 20% with Shift, snap to min/max with Home/End,
  // PageUp/PageDown for coarse 20% jumps. Closes the DESIGN.md "Bundle
  // alignment" §Trail item ("Arrow-key slider nudge") at the operator
  // surface — bundle uses native <input type=range> which gets nudge for
  // free; our custom role="slider" div needs the handler explicitly.
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

<!-- The interactive (role="slider" + tabindex + aria-value*) branch
     and the display-only (role="presentation" only) branch are
     duplicated so the static-analysis pass in vite-plugin-svelte
     can verify each branch independently — the previous ternary
     form tripped `a11y_no_noninteractive_tabindex`. The CSS class
     surface stays single-source per .fader-track. -->
<div class="fader" class:mod-halo={modulated} class:fader--disabled={disabled}>
  {#if isSlider}
    <div
      bind:this={trackEl}
      class="fader-track"
      role="slider"
      tabindex="0"
      aria-valuemin={0}
      aria-valuemax={1}
      aria-valuenow={pct}
      aria-label={label ?? 'fader'}
      aria-disabled={disabled ? 'true' : undefined}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
      onkeydown={onKeyDown}
    >
      <div class="fader-fill" style="height: {pct * 100}%"></div>
      <div class="fader-cap" style="bottom: calc({pct * 100}% - 4px)"></div>
    </div>
  {:else}
    <div bind:this={trackEl} class="fader-track" role="presentation">
      <div class="fader-fill" style="height: {pct * 100}%"></div>
      <div class="fader-cap" style="bottom: calc({pct * 100}% - 4px)"></div>
    </div>
  {/if}
  {#if label}<div class="fader-label">{label}</div>{/if}
</div>

<style>
  .fader {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 7px;
    min-width: 40px;
    position: relative;
    padding-top: 4px;
  }
  .fader--disabled {
    opacity: 0.5;
    pointer-events: none;
  }
  .fader-track {
    position: relative;
    width: 10px;
    height: 84px;
    background: var(--color-bg-1);
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-1);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.6);
    touch-action: none;
    user-select: none;
  }
  .fader-track[role='slider'] {
    cursor: ns-resize;
  }
  .fader-track:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .fader-fill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--tint, var(--color-primary));
    border-radius: 0 0 1px 1px;
    opacity: 0.95;
    pointer-events: none;
  }
  .fader-cap {
    position: absolute;
    left: -5px;
    right: -5px;
    height: 8px;
    background: var(--color-text-1);
    border: 1px solid var(--color-line-3);
    border-radius: 1px;
    pointer-events: none;
  }
  .fader-label {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-caps-md);
    font-weight: var(--font-weight-label);
    color: var(--color-text-1);
    text-align: center;
    line-height: 1.15;
    max-width: 58px;
  }
  .mod-halo {
    position: relative;
  }
  .mod-halo::after {
    content: '';
    position: absolute;
    inset: -3px;
    border-radius: var(--radius-xs);
    box-shadow:
      0 0 0 1px var(--color-signal-magenta),
      0 0 calc(12px * var(--glow)) var(--color-signal-magenta);
    pointer-events: none;
    opacity: 0.6;
    animation: modpulse 1.4s infinite;
  }
  @keyframes modpulse {
    0%,
    100% {
      opacity: 0.6;
    }
    50% {
      opacity: 0.18;
    }
  }
</style>
