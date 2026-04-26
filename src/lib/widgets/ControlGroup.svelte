<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-8 visual primitive: control-group container. Renders a header band
  // with the group's name and a category-tinted top border, then yields to a
  // children snippet. The tint maps `name` to a `--color-cat-*` token; names
  // outside the canonical set fall back to `--color-cat-general`. Callers
  // may pass an explicit `tint` prop (a CSS colour or `var(--…)`) to override
  // the name-based lookup — required for the v0.2 bundle's PerformView whose
  // group names ("action", "effects", "speed", "push", "dive", "mirror",
  // "video", "limit colors", "general") do not all overlap with the
  // canonical `--color-cat-*` keys; the bundle JSX passes `tint` directly
  // per-group, so the W18 PR1 Perform port mirrors that contract.
  import type { Snippet } from 'svelte';

  let { name, tint, children }: { name: string; tint?: string; children: Snippet } = $props();

  const TINTS: Record<string, string> = {
    color: 'var(--color-cat-color)',
    motion: 'var(--color-cat-motion)',
    fx: 'var(--color-cat-fx)',
    media: 'var(--color-cat-media)',
    limit: 'var(--color-cat-limit)',
    geom: 'var(--color-cat-geom)',
  };
  const resolvedTint = $derived(tint ?? TINTS[name.toLowerCase()] ?? 'var(--color-cat-general)');
</script>

<div class="group" style="--tint: {resolvedTint}">
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
