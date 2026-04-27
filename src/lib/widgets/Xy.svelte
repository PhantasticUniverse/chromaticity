<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-32 visual primitive: 2D xy pad. Display-only (no pointer
  // wiring) per the W8 Fader / Knob / Toggle / Swatch precedent;
  // ports the bundle's `chroma_inspector.jsx::ChromaCtrlBody` `xy`
  // branch (lines 151-167). The dot's normalized position is derived
  // from `value` against `min` / `max` so authors can pass either a
  // [-1, 1]-range Vec2 or a [0, 1]-range Vec2 without changing the
  // widget. Pointer-drag to update arrives with the W11 IPC scene
  // editor wave (out of scope here).
  let {
    value,
    min = [-1, -1] as const,
    max = [1, 1] as const,
  }: {
    value: readonly [number, number];
    min?: readonly [number, number];
    max?: readonly [number, number];
  } = $props();

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
</script>

<div class="xy">
  <div class="xy__pad" role="img" aria-label="xy pad — x {fmt(value[0])} y {fmt(value[1])}">
    <div class="xy__dot" style:left="{nxClamped}%" style:top="{nyClamped}%"></div>
  </div>
  <div class="xy__readout mono" aria-hidden="true">
    <span>x {fmt(value[0])}</span>
    <span>y {fmt(value[1])}</span>
  </div>
</div>

<style>
  .xy {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 56px;
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
  .xy__dot {
    position: absolute;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--tint, var(--color-primary));
    transform: translate(-50%, -50%);
    box-shadow: 0 0 8px var(--tint, var(--color-primary));
  }
  .xy__readout {
    display: flex;
    gap: 6px;
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }
</style>
