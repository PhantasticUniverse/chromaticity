<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-8 visual primitive: vertical fader. Display-only — no drag, no IPC.
  // W9 will wire pointer / IPC behaviour. The optional modulation halo is a
  // pure-CSS @keyframes pulse on a ::after pseudo-element so no JS tick loop
  // is required.
  let {
    value,
    label,
    modulated = false,
  }: { value: number; label?: string; modulated?: boolean } = $props();

  const pct = $derived(Math.max(0, Math.min(1, value)));
</script>

<div class="fader" class:mod-halo={modulated} role="presentation">
  <div class="fader-track">
    <div class="fader-fill" style="height: {pct * 100}%"></div>
    <div class="fader-cap" style="bottom: calc({pct * 100}% - 4px)"></div>
  </div>
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
  .fader-track {
    position: relative;
    width: 10px;
    height: 84px;
    background: var(--color-bg-1);
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-1);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.6);
  }
  .fader-fill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--tint, var(--color-primary));
    border-radius: 0 0 1px 1px;
    opacity: 0.95;
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
