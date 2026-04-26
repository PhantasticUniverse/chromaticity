<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-8 visual primitive: control-group container. Renders a header band
  // with the group's name and a category-tinted top border, then yields to a
  // children snippet. The tint maps `name` to a `--color-cat-*` token; names
  // outside the canonical set fall back to `--color-cat-general`.
  import type { Snippet } from 'svelte';

  let { name, children }: { name: string; children: Snippet } = $props();

  const TINTS: Record<string, string> = {
    color: 'var(--color-cat-color)',
    motion: 'var(--color-cat-motion)',
    fx: 'var(--color-cat-fx)',
    media: 'var(--color-cat-media)',
    limit: 'var(--color-cat-limit)',
    geom: 'var(--color-cat-geom)',
  };
  const tint = $derived(TINTS[name.toLowerCase()] ?? 'var(--color-cat-general)');
</script>

<div class="group" style="--tint: {tint}">
  <div class="group-head">
    <span class="group-name">{name}</span>
  </div>
  <div class="group-grid">{@render children()}</div>
</div>

<style>
  .group {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-top: 2px solid var(--tint);
    border-radius: var(--radius-md);
    padding: var(--space-sm) var(--space-md) var(--space-md);
    position: relative;
    box-shadow: var(--shadow-panel);
  }
  .group-head {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    margin-bottom: var(--space-sm);
  }
  .group-name {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--tint);
  }
  .group-grid {
    display: flex;
    gap: var(--space-md);
    align-items: flex-end;
  }
</style>
