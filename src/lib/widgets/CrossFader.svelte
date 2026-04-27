<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  import type { PointerEventHandler } from 'svelte/elements';

  let {
    value = 0.5,
    labels = ['A', 'B'],
    onChange,
    disabled = false,
  }: {
    value?: number;
    labels?: [string, string];
    onChange?: (v: number) => void;
    disabled?: boolean;
  } = $props();

  let trackEl = $state<HTMLDivElement | null>(null);
  let dragging = $state(false);

  const pct = $derived(Math.max(0, Math.min(1, value)) * 100);

  function valueFromClientX(clientX: number): number {
    if (!trackEl) return value;
    const rect = trackEl.getBoundingClientRect();
    if (rect.width <= 0) return value;
    const raw = (clientX - rect.left) / rect.width;
    return Math.max(0, Math.min(1, raw));
  }

  const onPointerDown: PointerEventHandler<HTMLDivElement> = (event) => {
    if (disabled) return;
    event.preventDefault();
    dragging = true;
    const target = event.currentTarget;
    target.setPointerCapture(event.pointerId);
    onChange?.(valueFromClientX(event.clientX));
  };

  const onPointerMove: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!dragging || disabled) return;
    onChange?.(valueFromClientX(event.clientX));
  };

  const onPointerUp: PointerEventHandler<HTMLDivElement> = (event) => {
    if (!dragging) return;
    dragging = false;
    const target = event.currentTarget;
    if (target.hasPointerCapture(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }
  };
</script>

<div class="xfade-wrap" class:xfade-wrap--disabled={disabled} style="--xfade-pos: {pct}%">
  <div
    bind:this={trackEl}
    class="xfade"
    role="slider"
    tabindex={disabled ? -1 : 0}
    aria-valuemin={0}
    aria-valuemax={1}
    aria-valuenow={Math.max(0, Math.min(1, value))}
    aria-label="Crossfader {labels[0]} to {labels[1]}"
    aria-disabled={disabled ? 'true' : 'false'}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
  >
    <div class="xfade-thumb" aria-hidden="true"></div>
  </div>
  <div class="xfade-labels" aria-hidden="true">
    <span>{labels[0]}</span>
    <span>{labels[1]}</span>
  </div>
</div>

<style>
  .xfade-wrap {
    padding: 4px 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
    min-width: 120px;
    user-select: none;
  }
  .xfade-wrap--disabled {
    opacity: 0.5;
    pointer-events: none;
  }
  .xfade {
    position: relative;
    height: 20px;
    background: linear-gradient(
      90deg,
      var(--color-bg-3) 0%,
      var(--color-bg-2) 50%,
      var(--color-bg-3) 100%
    );
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-2);
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.6);
    cursor: ew-resize;
    touch-action: none;
  }
  .xfade:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .xfade-thumb {
    position: absolute;
    top: -3px;
    bottom: -3px;
    left: var(--xfade-pos, 50%);
    width: 14px;
    transform: translateX(-50%);
    background: var(--color-primary);
    border: 1px solid var(--color-line-3);
    border-radius: var(--radius-xs);
    box-shadow:
      0 2px 4px rgba(0, 0, 0, 0.6),
      0 0 calc(6px * var(--glow, 1)) var(--color-primary);
    pointer-events: none;
  }
  .xfade-labels {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
  }
</style>
