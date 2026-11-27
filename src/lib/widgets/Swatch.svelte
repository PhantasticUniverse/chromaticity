<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-8 visual primitive: colour swatch. Display-only. The `color` prop is
  // a free-form CSS colour string; the caller is responsible for theme
  // alignment (e.g. passing `var(--color-primary)` rather than a literal hex).
  let { color, label }: { color: string; label?: string } = $props();
</script>

<div class="swatch-wrap" role="presentation">
  <div class="swatch" style="background: {color}"></div>
  {#if label}<div class="swatch-label">{label}</div>{/if}
</div>

<style>
  .swatch-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-xs);
    min-width: 46px;
  }
  .swatch {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid var(--color-bg-3);
    box-shadow: 0 0 0 1px var(--color-line-2);
  }
  .swatch-label {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-caps-md);
    font-weight: var(--font-weight-label);
    color: var(--color-text-1);
    text-align: center;
    line-height: 1.15;
  }
</style>
