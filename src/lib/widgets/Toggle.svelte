<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-8 visual primitive: on/off switch. Display-only. W9 will wire click
  // / IPC behaviour. Vertical default mirrors the bundle's vertical-fader
  // sibling layout; the horizontal variant is a pill switch with thumb.
  let {
    value,
    label,
    orientation = 'vertical',
  }: { value: boolean; label?: string; orientation?: 'vertical' | 'horizontal' } = $props();
</script>

<div class="tog" class:on={value} class:tog-h={orientation === 'horizontal'} role="presentation">
  <div class="tog-switch"></div>
  {#if label}<div class="tog-label">{label}</div>{/if}
</div>

<style>
  .tog {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 7px;
    min-width: 40px;
    position: relative;
    padding-top: 4px;
  }
  .tog-switch {
    position: relative;
    width: 10px;
    height: 38px;
    background: var(--color-bg-1);
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-1);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.6);
  }
  .tog-switch::before {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    background: var(--color-primary);
    opacity: 0.95;
    top: 100%;
    bottom: 0;
    transition:
      top 0.1s ease-out,
      bottom 0.1s ease-out;
  }
  .tog-switch::after {
    content: '';
    position: absolute;
    left: -5px;
    right: -5px;
    height: 8px;
    bottom: -4px;
    background: var(--color-text-1);
    border: 1px solid var(--color-line-3);
    border-radius: 1px;
    transition:
      top 0.1s ease-out,
      bottom 0.1s ease-out;
  }
  .tog.on .tog-switch::before {
    top: 0;
    bottom: 0;
  }
  .tog.on .tog-switch::after {
    top: -4px;
    bottom: auto;
  }
  .tog-label {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-caps-md);
    font-weight: var(--font-weight-label);
    color: var(--color-text-1);
    text-align: center;
    line-height: 1.15;
  }
  .tog.tog-h {
    flex-direction: row;
    align-items: center;
    gap: var(--space-md);
    padding-top: 0;
  }
  .tog.tog-h .tog-switch {
    width: 32px;
    height: 18px;
    border-radius: var(--radius-full);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.5);
  }
  .tog.tog-h .tog-switch::before {
    display: none;
  }
  .tog.tog-h .tog-switch::after {
    position: absolute;
    top: 2px;
    left: 2px;
    right: auto;
    bottom: auto;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-text-0);
    border: 1px solid var(--color-line-3);
    transform: translateX(0);
    transition: transform 0.15s ease;
  }
  .tog.tog-h.on .tog-switch {
    background: var(--color-primary);
    border-color: var(--color-primary);
  }
  .tog.tog-h.on .tog-switch::after {
    transform: translateX(14px);
    background: var(--color-on-primary);
    border-color: var(--color-on-primary);
  }
</style>
