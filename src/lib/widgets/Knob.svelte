<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-8 visual primitive: rotary knob. Display-only — no drag, no scrub,
  // no detents. Indicator rotates -135° to +135° (270° sweep). W9 will wire
  // pointer / IPC behaviour.
  let {
    value,
    label,
    showValue = false,
  }: { value: number; label?: string; showValue?: boolean } = $props();

  const pct = $derived(Math.max(0, Math.min(1, value)));
  const rot = $derived(-135 + pct * 270);
</script>

<div class="knob" role="presentation">
  <div class="knob-dial" style="--pct: {pct}; --rot: {rot}deg">
    <div class="knob-arc"></div>
    <div class="knob-ind"></div>
    {#if showValue}<div class="knob-val">{value.toFixed(2)}</div>{/if}
  </div>
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
  }
  .knob-val {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
    white-space: nowrap;
    pointer-events: none;
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
