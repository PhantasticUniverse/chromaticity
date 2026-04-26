<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-12 PR3 left navigation rail. Renders the 12-icon list seen in
  // every wireframe (`.playwright-mcp/wireframe-reference/01-…07-…png`):
  // Perform / Library / Author / Patch / Playlist / Media / MIDI / OSC /
  // Import / Settings in the top section, Help / Account at the bottom.
  //
  // Only 4 routes (perform / library / author / settings) are wired in
  // W12; the other 8 render as `aria-disabled` buttons with a tooltip
  // signalling W13+ landing. Keeping them visible (rather than hidden)
  // mirrors the wireframe layout so the operator's spatial memory of
  // "Library is the second icon" is established now.
  //
  // Icons are Unicode glyphs in W12 — proper SVG icons are W13+ and gated
  // on the bundle's icon-set licensing review. Glyphs were chosen for
  // legibility across the three themes (pulse / atelier / console);
  // each glyph evokes the bundle JSX semantics without claiming
  // pixel-parity with the wireframe.
  import { routeStore, type RouteName } from './stores/route.svelte';

  interface NavEntry {
    readonly route: RouteName;
    readonly label: string;
    readonly glyph: string;
  }

  interface DisabledEntry {
    readonly label: string;
    readonly glyph: string;
    readonly section: 'top' | 'bottom';
  }

  // Order mirrors `.playwright-mcp/wireframe-reference/README.md` §sidebar.
  const NAV_TOP: readonly (NavEntry | DisabledEntry)[] = [
    { route: 'perform', label: 'Perform', glyph: '▶' },
    { route: 'library', label: 'Library', glyph: '▦' },
    { route: 'author', label: 'Author', glyph: '✎' },
    { label: 'Patch', glyph: '◫', section: 'top' },
    { label: 'Playlist', glyph: '☰', section: 'top' },
    { label: 'Media', glyph: '◉', section: 'top' },
    { label: 'MIDI', glyph: '♪', section: 'top' },
    { label: 'OSC', glyph: '⌘', section: 'top' },
    { label: 'Import', glyph: '↓', section: 'top' },
    { route: 'settings', label: 'Settings', glyph: '⚙' },
  ] as const;

  const NAV_BOTTOM: readonly DisabledEntry[] = [
    { label: 'Help', glyph: '?', section: 'bottom' },
    { label: 'Account', glyph: '◐', section: 'bottom' },
  ] as const;

  function isWired(entry: NavEntry | DisabledEntry): entry is NavEntry {
    return 'route' in entry;
  }
</script>

<nav class="sidebar-rail" aria-label="Primary navigation">
  <ul class="sidebar-rail__section sidebar-rail__section--top">
    {#each NAV_TOP as entry (entry.label)}
      <li>
        {#if isWired(entry)}
          <button
            type="button"
            class="sidebar-rail__btn"
            class:active={routeStore.current === entry.route}
            aria-current={routeStore.current === entry.route ? 'page' : undefined}
            title={entry.label}
            onclick={() => routeStore.set(entry.route)}
          >
            <span class="sidebar-rail__glyph" aria-hidden="true">{entry.glyph}</span>
            <span class="sidebar-rail__label">{entry.label}</span>
          </button>
        {:else}
          <button
            type="button"
            class="sidebar-rail__btn"
            aria-disabled="true"
            disabled
            title="{entry.label} — coming in W13+"
          >
            <span class="sidebar-rail__glyph" aria-hidden="true">{entry.glyph}</span>
            <span class="sidebar-rail__label">{entry.label}</span>
          </button>
        {/if}
      </li>
    {/each}
  </ul>
  <ul class="sidebar-rail__section sidebar-rail__section--bottom">
    {#each NAV_BOTTOM as entry (entry.label)}
      <li>
        <button
          type="button"
          class="sidebar-rail__btn"
          aria-disabled="true"
          disabled
          title="{entry.label} — coming in W13+"
        >
          <span class="sidebar-rail__glyph" aria-hidden="true">{entry.glyph}</span>
          <span class="sidebar-rail__label">{entry.label}</span>
        </button>
      </li>
    {/each}
  </ul>
</nav>

<style>
  /* Token-driven scoped styles. No new tokens added — `--color-bg-1`
   * (sidebar surface), `--color-bg-3` (hover), `--color-primary` +
   * `--color-on-primary` (active) cover every state. */
  .sidebar-rail {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    background: var(--color-bg-1);
    border-right: 1px solid var(--color-line-2);
    padding: var(--space-sm) 0;
    overflow-y: auto;
  }
  .sidebar-rail__section {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .sidebar-rail__btn {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    background: transparent;
    color: var(--color-text-2);
    border: none;
    border-left: 2px solid transparent;
    padding: var(--space-sm) var(--space-xs);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-caps-sm);
    letter-spacing: var(--letter-spacing-caps-sm);
    text-transform: uppercase;
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease,
      border-color 0.12s ease;
  }
  .sidebar-rail__btn:not(:disabled):hover {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }
  .sidebar-rail__btn:not(:disabled):focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }
  .sidebar-rail__btn.active {
    background: var(--color-bg-3);
    color: var(--color-primary);
    border-left-color: var(--color-primary);
  }
  .sidebar-rail__btn:disabled {
    color: var(--color-text-4);
    cursor: not-allowed;
  }
  .sidebar-rail__glyph {
    font-size: var(--font-size-display-md);
    line-height: var(--line-height-tight);
    color: inherit;
  }
  .sidebar-rail__label {
    font-family: var(--font-family-mono);
    font-size: 9px;
    letter-spacing: 0.04em;
    color: inherit;
  }
</style>
