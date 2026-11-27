<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-32 visual primitive: momentary trigger button. Display-only
  // (no click wiring) per the W8 sibling precedent. Ports the bundle's
  // `chroma_inspector.jsx::ChromaCtrlBody` `bang` branch (lines
  // 138-149). The bundle distinguishes a "counter" mode that surfaces
  // the accumulated count next to a "CLICK +" label; this primitive
  // takes a `mode` prop to switch between the two presentations
  // verbatim. Real `onpress` handling lands with the W11 IPC scene
  // editor wave.
  let {
    label,
    mode = 'trigger',
    count = 0,
  }: {
    label?: string;
    mode?: 'trigger' | 'counter';
    count?: number;
  } = $props();

  const buttonLabel = $derived(label ?? (mode === 'counter' ? 'CLICK +' : 'TRIGGER'));
</script>

<div class="bang">
  <button type="button" class="bang__btn mono" disabled aria-disabled="true">
    {buttonLabel}
  </button>
  {#if mode === 'counter'}
    <span class="bang__count mono" aria-label="bang count">×{count}</span>
  {/if}
</div>

<style>
  .bang {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .bang__btn {
    background: var(--color-bg-1);
    color: var(--tint, var(--color-text-1));
    border: 1px solid var(--tint, var(--color-line-2));
    border-radius: var(--radius-sm);
    padding: 4px 10px;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: not-allowed;
    opacity: 0.85;
  }
  .bang__count {
    color: var(--tint, var(--color-text-2));
    font-size: var(--font-size-mono-sm);
  }
</style>
