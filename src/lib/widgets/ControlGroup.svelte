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
  //
  // Wave 2.3 Lane Q.2 (G.6) — per-row Lock / Reset / Random affordances
  // ported from the bundle's `ControlGroup` (`components.jsx` lines
  // 147-161). Driven by the new `actions` prop (default `false` to
  // preserve the W18 PR1 / Settings-view chrome; Perform's Scene + Meta
  // panels opt in by passing `actions`). Buttons are visual-only stubs
  // tracking the same "controls dispatch but visual pattern doesn't
  // change" posture already in place across Scene Controls — matches
  // the Wave 2.1 Lane A.2 IPC-vs-render boundary.
  import type { Snippet } from 'svelte';

  import Icon from '../Icon.svelte';

  let {
    name,
    tint,
    actions = false,
    children,
  }: { name: string; tint?: string; actions?: boolean; children: Snippet } = $props();

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
    {#if actions}
      <span class="group-actions" aria-hidden="false">
        <button type="button" class="group-action" title="Lock controls" aria-label="Lock controls">
          <Icon name="lock" size={10} />
        </button>
        <button
          type="button"
          class="group-action"
          title="Reset to defaults"
          aria-label="Reset to defaults"
        >
          <Icon name="undo" size={10} />
        </button>
        <button type="button" class="group-action" title="Randomize" aria-label="Randomize">
          <Icon name="random" size={10} />
        </button>
      </span>
    {/if}
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
    flex: 1;
    min-width: 0;
  }
  .group-actions {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }
  .group-action {
    appearance: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    background: transparent;
    border: 0;
    color: var(--color-text-3);
    cursor: pointer;
    padding: 0;
    border-radius: var(--radius-xs);
    transition:
      color 0.12s ease,
      background-color 0.12s ease;
  }
  .group-action:hover {
    color: var(--color-text-0);
    background: var(--color-bg-3);
  }
  .group-action:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .group-grid {
    display: flex;
    gap: var(--space-md);
    align-items: flex-end;
  }
</style>
