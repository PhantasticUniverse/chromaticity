<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Operator hotkey overlay. Toggled by `?` from `src/App.svelte`. Renders the
  // canonical map from `./data/hotkeys.ts` with a search filter, ESC-to-close,
  // autofocused search input, focus restoration on close, and an inline
  // focus-trap across the search field + close button. The durable focus-trap
  // primitive (`lib/a11y/focusTrap.ts`) is deferred to W9; this minimal trap
  // is intentional given there are only two tabbable elements in the dialog.
  import { HOTKEYS, type HotkeyGroup } from './data/hotkeys';

  let { open, onCloseRequest }: { open: boolean; onCloseRequest: () => void } = $props();

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
      // Defer focus until the input is mounted in the DOM.
      queueMicrotask(() => searchEl?.focus());
    } else if (previouslyFocused) {
      previouslyFocused.focus();
      previouslyFocused = null;
    }
  });

  const filteredGroups = $derived.by<readonly HotkeyGroup[]>(() => {
    const q = query.trim().toLowerCase();
    if (!q) return HOTKEYS;
    return HOTKEYS.map((g) => ({
      ...g,
      rows: g.rows.filter((r) => {
        const hay = `${r.action} ${r.keys.join(' ')} ${r.osc ?? ''} ${r.glsl ?? ''}`.toLowerCase();
        return hay.includes(q);
      }),
    })).filter((g) => g.rows.length > 0);
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onCloseRequest();
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
</script>

{#if open}
  <div
    class="hotkey-overlay"
    role="dialog"
    aria-modal="true"
    aria-label="Operator hotkey map"
    tabindex="-1"
    bind:this={dialogEl}
    onkeydown={handleKeydown}
  >
    <div class="hotkey-overlay__panel">
      <header>
        <h2>Hotkeys</h2>
        <button type="button" onclick={onCloseRequest} aria-label="Close hotkey overlay">
          ×
        </button>
      </header>
      <input
        type="search"
        placeholder="Filter…"
        bind:value={query}
        bind:this={searchEl}
        aria-label="Filter hotkeys"
      />
      <div class="hotkey-overlay__groups">
        {#each filteredGroups as group (group.title)}
          <section>
            <h3>{group.title}</h3>
            {#if group.note}
              <p class="note">{group.note}</p>
            {/if}
            <table>
              <!--
                TODO(scale): adopt @tanstack/svelte-virtual (or equivalent) when hotkey
                vocabulary exceeds ~200 rows per group. Currently 57 total rows
                across all groups; full DOM render is fine. Pre-flag for future
                hotkey-vocabulary expansion (e.g., per-cohort macro packs).
              -->
              <tbody>
                {#each group.rows as row, i (i)}
                  <tr
                    class:planned={row.planned === true}
                    title={row.planned ? 'Planned — not available yet' : undefined}
                  >
                    <td class="keys">
                      {#each row.keys as combo, ki (ki)}
                        {#if ki > 0}<span class="sep" aria-hidden="true">/</span>{/if}
                        {#each combo.split('+') as part, pi (pi)}
                          {#if pi > 0}<span class="plus" aria-hidden="true">+</span>{/if}
                          <kbd>{part}</kbd>
                        {/each}
                      {/each}
                    </td>
                    <td class="action">
                      {row.action}
                      {#if row.planned}<span class="planned-tag mono">planned</span>{/if}
                    </td>
                    <td class="osc">{row.osc ?? ''}</td>
                    <td class="glsl">{row.glsl ?? ''}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </section>
        {/each}
        {#if filteredGroups.length === 0}
          <p class="empty">no matches for "{query}"</p>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .hotkey-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .hotkey-overlay__panel {
    background: var(--color-bg-2);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    width: min(56rem, 92vw);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .hotkey-overlay__panel header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .hotkey-overlay__panel h2 {
    font-size: var(--font-size-display-md);
    margin: 0;
  }
  .hotkey-overlay__panel header button {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: 2px var(--space-sm);
    cursor: pointer;
    font-size: var(--font-size-body-md);
  }
  .hotkey-overlay__panel input[type='search'] {
    background: var(--color-bg-3);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    padding: var(--space-xs) var(--space-sm);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
  }
  .hotkey-overlay__panel input[type='search']:focus,
  .hotkey-overlay__panel header button:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }
  .hotkey-overlay__groups {
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    font-size: var(--font-size-body-sm);
  }
  .hotkey-overlay__groups h3 {
    font-size: var(--font-size-display-sm);
    margin: 0 0 var(--space-xs) 0;
    color: var(--color-text-1);
  }
  .hotkey-overlay__groups .note {
    margin: 0 0 var(--space-sm) 0;
    color: var(--color-text-2);
  }
  .hotkey-overlay__groups table {
    width: 100%;
    border-collapse: collapse;
  }
  .hotkey-overlay__groups td {
    padding: var(--space-xs) var(--space-sm);
    border-bottom: 1px solid var(--color-line-1);
    vertical-align: top;
  }
  .hotkey-overlay__groups .keys {
    white-space: nowrap;
  }
  .hotkey-overlay__groups .action {
    color: var(--color-text-1);
  }
  .hotkey-overlay__groups .osc,
  .hotkey-overlay__groups .glsl {
    font-family: var(--font-family-mono);
    color: var(--color-text-2);
  }
  .hotkey-overlay__groups kbd {
    display: inline-block;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    padding: 2px var(--space-xs);
    color: var(--color-text-0);
  }
  .hotkey-overlay__groups .sep,
  .hotkey-overlay__groups .plus {
    color: var(--color-text-3);
    margin: 0 var(--space-xs);
  }
  .hotkey-overlay__groups .empty {
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
  }
  tr.planned {
    opacity: 0.5;
  }
  .planned-tag {
    margin-left: 6px;
    padding: 0 4px;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-3, var(--color-text-2));
  }
</style>
