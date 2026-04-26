<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-8 visual primitive: empty-state surface for views with no content.
  // Ported from the bundle's `empty_states.jsx::EmptyState`. Renders a
  // centered orbit-ring backdrop, an optional inline glyph, a title, an
  // optional body paragraph, and an `actions` snippet slot. The snippet
  // owns its focusable buttons; the primitive itself is non-focusable.
  import type { Snippet } from 'svelte';

  let {
    title,
    body,
    iconName,
    actions,
  }: {
    title: string;
    body?: string;
    iconName?: string;
    actions?: Snippet;
  } = $props();
</script>

<div class="empty-state" role="presentation">
  <div class="empty-state-art" aria-hidden="true">
    <svg viewBox="0 0 160 160" class="empty-orbit">
      <circle cx="80" cy="80" r="70" class="empty-orbit-ring" />
      <circle cx="80" cy="80" r="54" class="empty-orbit-ring-2" />
    </svg>
    {#if iconName}
      <svg viewBox="0 0 24 24" class="empty-state-icon" data-icon={iconName}>
        <circle cx="12" cy="12" r="6" />
      </svg>
    {/if}
  </div>
  <h2 class="empty-state-title">{title}</h2>
  {#if body}
    <p class="empty-state-body">{body}</p>
  {/if}
  {#if actions}
    <div class="empty-state-actions">{@render actions()}</div>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-md);
    padding: var(--space-lg);
    background: var(--color-bg-2);
    color: var(--color-text-1);
    border-radius: var(--radius-md);
    max-width: 32rem;
    margin: 0 auto;
  }
  .empty-state-art {
    position: relative;
    width: 160px;
    height: 160px;
  }
  .empty-orbit {
    width: 100%;
    height: 100%;
    fill: none;
    stroke: var(--color-line-2);
    opacity: 0.6;
  }
  .empty-orbit-ring {
    stroke-dasharray: 2 6;
  }
  .empty-orbit-ring-2 {
    stroke-dasharray: 1 8;
    opacity: 0.5;
  }
  .empty-state-icon {
    position: absolute;
    inset: 0;
    margin: auto;
    width: 28px;
    height: 28px;
    fill: var(--color-text-2);
  }
  .empty-state-title {
    margin: 0;
    font-size: var(--font-size-display-md);
    color: var(--color-text-0);
  }
  .empty-state-body {
    margin: 0;
    font-size: var(--font-size-body-md);
    line-height: var(--line-height-body);
    color: var(--color-text-2);
    max-width: 28rem;
  }
  .empty-state-actions {
    display: flex;
    gap: var(--space-sm);
    flex-wrap: wrap;
    justify-content: center;
  }
</style>
