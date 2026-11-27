<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  import {
    DEFAULT_COMMAND_PALETTE_ACTIONS,
    type CommandPaletteAction,
  } from './command-palette.fixtures';

  // Wave-29 PR1 widget primitive: Cmd+K command palette overlay. Bundle
  // source: `chromaticity-planning/ui-predev-bundle-v0.2-beta/extras.jsx`
  // lines 124-166 (`CommandPalette` component). Mounted in `AppShell` at
  // z-index 1010 (one layer above HotkeyOverlay at 1000) so the two can
  // stack without occluding each other if both happen to be open.
  //
  // Frozen-frame contract: the seven default actions ("Launch scene…" →
  // "Open offline docs") match the bundle list verbatim. Where an action
  // has a real W12+ destination (Library / Settings) the AppShell mount
  // attaches an `onSelect` that routes via `routeStore`; the rest are
  // pure dismiss-on-click no-ops pending Phase-3+/4a
  // `EV-INTEGRATION-MATRIX` wiring (Snap recall, OSC address surfacing,
  // support-bundle generation, offline-docs dispatch).
  //
  // Focus-trap idiom matches `HotkeyOverlay.svelte` (W8 PR2) directly —
  // capture the previously focused element on open, autofocus the search
  // input via queueMicrotask, restore focus on close, and trap Tab /
  // Shift+Tab between the first and last tabbable inside the dialog.
  // The durable focus-trap primitive (`lib/a11y/focusTrap.ts`) is still
  // deferred. With only a search input + close button + N action rows
  // the inline trap is bounded and predictable.

  let {
    open = false,
    onClose,
    commands = DEFAULT_COMMAND_PALETTE_ACTIONS,
  }: {
    open?: boolean;
    onClose: () => void;
    commands?: readonly CommandPaletteAction[];
  } = $props();

  let query = $state('');
  let dialogEl: HTMLDivElement | undefined = $state();
  let searchEl: HTMLInputElement | undefined = $state();

  // Module-state ref (NOT $state) — restoration target survives the
  // close-side $effect cleanup and need not trigger reactivity.
  let previouslyFocused: HTMLElement | null = null;

  $effect(() => {
    if (open) {
      const active = document.activeElement;
      previouslyFocused = active instanceof HTMLElement ? active : null;
      query = '';
      queueMicrotask(() => searchEl?.focus());
    } else if (previouslyFocused) {
      previouslyFocused.focus();
      previouslyFocused = null;
    }
  });

  const filtered = $derived.by<readonly CommandPaletteAction[]>(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return commands;
    return commands.filter((a) => `${a.label} ${a.meta}`.toLowerCase().includes(needle));
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== 'Tab' || !dialogEl) return;
    const tabbables = Array.from(dialogEl.querySelectorAll<HTMLElement>('input, button')).filter(
      (el) => !el.hasAttribute('disabled'),
    );
    if (tabbables.length === 0) return;
    const first = tabbables[0];
    const last = tabbables[tabbables.length - 1];
    if (!first || !last) return;
    const activeEl = document.activeElement;
    if (event.shiftKey && activeEl === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && activeEl === last) {
      event.preventDefault();
      first.focus();
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) onClose();
  }

  function handleSelect(action: CommandPaletteAction) {
    action.onSelect?.();
    onClose();
  }
</script>

{#if open}
  <div
    class="command-palette__backdrop"
    role="presentation"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
  >
    <div
      class="command-palette"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabindex="-1"
      bind:this={dialogEl}
    >
      <header class="command-palette__head">
        <span class="command-palette__title">Command Palette</span>
        <button
          type="button"
          class="command-palette__close"
          onclick={onClose}
          aria-label="Close command palette">✕</button
        >
      </header>
      <input
        type="search"
        class="command-palette__search"
        placeholder="search scenes, presets, settings, help…"
        bind:value={query}
        bind:this={searchEl}
        aria-label="Search commands"
      />
      <div class="command-palette__scroll">
        {#each filtered as action (action.id)}
          <button type="button" class="command-palette__row" onclick={() => handleSelect(action)}>
            <span class="command-palette__row-label">{action.label}</span>
            <span class="command-palette__row-meta">{action.meta}</span>
          </button>
        {/each}
        {#if filtered.length === 0}
          <p class="command-palette__empty">no matching command</p>
        {/if}
      </div>
      <footer class="command-palette__foot">
        <kbd>Cmd</kbd>+<kbd>K</kbd> toggle · <kbd>Esc</kbd> close
      </footer>
    </div>
  </div>
{/if}

<style>
  .command-palette__backdrop {
    position: fixed;
    inset: 0;
    background: color-mix(in oklab, var(--color-bg-0) 80%, transparent);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
    z-index: 1010;
  }
  .command-palette {
    background: var(--color-bg-2);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    width: min(520px, 92vw);
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .command-palette__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .command-palette__title {
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: var(--font-size-caps-md);
    color: var(--color-text-1);
  }
  .command-palette__close {
    width: 22px;
    height: 22px;
    padding: 0;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    color: var(--color-text-1);
    cursor: pointer;
    font-size: var(--font-size-body-sm);
  }
  .command-palette__close:hover {
    color: var(--color-text-0);
  }
  .command-palette__close:focus,
  .command-palette__search:focus,
  .command-palette__row:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .command-palette__search {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: var(--space-xs) var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
  }
  .command-palette__scroll {
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .command-palette__row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-md);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-xs);
    color: var(--color-text-0);
    font: inherit;
    cursor: pointer;
    text-align: left;
    width: 100%;
  }
  .command-palette__row:hover {
    background: var(--color-bg-3);
    border-color: var(--color-line-1);
  }
  .command-palette__row-label {
    font-size: var(--font-size-body-sm);
  }
  .command-palette__row-meta {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }
  .command-palette__empty {
    margin: 0;
    padding: var(--space-md);
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    text-align: center;
  }
  .command-palette__foot {
    color: var(--color-text-3);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    border-top: 1px solid var(--color-line-1);
    padding-top: var(--space-sm);
  }
  .command-palette__foot kbd {
    display: inline-block;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    padding: 0 var(--space-xs);
    color: var(--color-text-1);
  }
</style>
