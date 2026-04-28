<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-40 visual primitive: horizontal segmented-button group. Bundle
  // source: `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/settings.jsx`
  // (`<div className="tab-seg">` adoption sites — Quantize Transitions /
  // Color Space / Multi-Display Mode) + `styles.css` lines 293-297 for
  // the canonical visuals. Display-only: caller owns selected state and
  // is notified via the optional `onChange` callback. Wrapper class is
  // `tab-segment` (not the bundle's `tab-seg`) so a future global
  // stylesheet port of `styles.css` can land without colliding with
  // Svelte's scoped class hashes here.
  let {
    options,
    value,
    onChange,
    ariaLabel = 'Segmented options',
  }: {
    options: readonly { readonly id: string; readonly label: string }[];
    value: string;
    onChange?: (id: string) => void;
    ariaLabel?: string;
  } = $props();

  function handleClick(id: string) {
    if (id !== value) onChange?.(id);
  }

  function handleKeydown(event: KeyboardEvent, idx: number) {
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
    if (options.length === 0) return;
    event.preventDefault();
    const delta = event.key === 'ArrowRight' ? 1 : -1;
    const nextIdx = (idx + delta + options.length) % options.length;
    const next = options[nextIdx];
    if (!next) return;
    onChange?.(next.id);
  }
</script>

<div class="tab-segment" role="tablist" aria-label={ariaLabel}>
  {#each options as opt, idx (opt.id)}
    {@const active = opt.id === value}
    <button
      type="button"
      class="tab"
      class:active
      role="tab"
      aria-selected={active}
      tabindex={active ? 0 : -1}
      onclick={() => handleClick(opt.id)}
      onkeydown={(e) => handleKeydown(e, idx)}
    >
      {opt.label}
    </button>
  {/each}
</div>

<style>
  .tab-segment {
    display: inline-flex;
    gap: 1px;
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: 2px;
  }
  .tab {
    padding: 3px 8px;
    background: transparent;
    border: none;
    border-bottom: 1px solid transparent;
    border-radius: var(--radius-xs);
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    cursor: pointer;
    transition: all 0.1s;
  }
  .tab:hover {
    color: var(--color-text-0);
  }
  .tab.active {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border-bottom: 1px solid var(--color-primary);
  }
  .tab:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
</style>
