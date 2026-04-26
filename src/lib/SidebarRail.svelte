<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-16 PR2 sidebar redesign — matches the v0.2 bundle's canonical
  // 13-icon nav set per the bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/Chromaticity.html`
  // lines 57-71, the `VIEWS` array). Order mirrors the bundle:
  // Perform / Library / Author / Audio Uniforms / Playlist / Sources /
  // MIDI / OSC / Import / Patch / Settings / License / Welcome in the
  // top section, Account / Help at the bottom.
  //
  // Wired routes after this PR (10 of 13 nav entries): perform / library
  // / author / audio-uniforms / playlist / sources / osc / settings /
  // license / welcome. The four routed-but-stub views (welcome /
  // sources / audio-uniforms / license) render an `EmptyState` until
  // their respective W17+ ports replace each stub with full content.
  //
  // Disabled entries (3): Patch (full node-graph port is W17+ scope per
  // brief), MIDI (W14+ candidate, view-port pending), Import (W14+
  // candidate, view-port pending). Bottom section (Account / Help)
  // remains disabled per W12 — both gate on entitlement / support-
  // bundle infrastructure that lands later.
  //
  // Wave-18 PR3 promotes the MIDI entry from disabled stub to routed
  // view, leaving Patch and Import as the only disabled top-section
  // entries. The full bundle port lives in `views/MIDI.svelte`.
  //
  // Icons remain Unicode glyphs in W16; SVG icon vendoring is W17+.
  // Glyphs picked for legibility across the three themes (pulse /
  // atelier / console). Per the auto-memory
  // `feedback_wireframes_normative_for_ui_ux.md`, the bundle JSX is the
  // canonical structural reference; the wireframe screenshots in
  // `v0.2-ui-wireframe-screenshots/pulse/` confirm the same nav set
  // visually but defer to the JSX when they appear to disagree.
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

  // Order mirrors the v0.2 bundle's `VIEWS` array.
  const NAV_TOP: readonly (NavEntry | DisabledEntry)[] = [
    { route: 'perform', label: 'Perform', glyph: '▶' },
    { route: 'library', label: 'Library', glyph: '▦' },
    { route: 'author', label: 'Author', glyph: '✎' },
    { route: 'audio-uniforms', label: 'Audio', glyph: '〰' },
    { route: 'playlist', label: 'Playlist', glyph: '☰' },
    { route: 'sources', label: 'Sources', glyph: '▣' },
    { route: 'midi', label: 'MIDI', glyph: '♪' },
    { route: 'osc', label: 'OSC', glyph: '⌘' },
    { label: 'Import', glyph: '↓', section: 'top' },
    { label: 'Patch', glyph: '◫', section: 'top' },
    { route: 'settings', label: 'Settings', glyph: '⚙' },
    { route: 'license', label: 'License', glyph: '◆' },
    { route: 'welcome', label: 'Welcome', glyph: '✦' },
  ] as const;

  const NAV_BOTTOM: readonly DisabledEntry[] = [
    { label: 'Account', glyph: '◐', section: 'bottom' },
    { label: 'Help', glyph: '?', section: 'bottom' },
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
            title="{entry.label} — coming in W17+"
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
          title="{entry.label} — coming in W17+"
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
