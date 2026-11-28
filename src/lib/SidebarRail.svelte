<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
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
  // Post-W19 status: every top-section sidebar entry (all 13) is now a
  // routed view with a full bundle port — Welcome, Sources, Audio
  // Uniforms, and License each carry their own "Locked deviations from
  // the bundle" sections in their respective `views/*.svelte` files.
  // The W18 PR3 / W19 PR1 / W19 PR2 sub-agents promoted MIDI / Patch /
  // Import from disabled stubs to routed views. The only disabled
  // entries that remain are the bottom-section Account / Help, both
  // gated on entitlement / support-bundle infrastructure that lands
  // later.
  //
  // Wave-19 PR3 swapped the Unicode glyph column for the vendored
  // `Icon.svelte` SVG primitive. Icon-name mapping follows the bundle's
  // `VIEWS` array verbatim (Chromaticity.html lines 57-71): perform /
  // library / edit / audio / list / media / midi / link / plus / cpu /
  // settings / lock / heart, plus user / help in the bottom section.
  // Each name resolves to a 24×24 viewBox stroke icon that inherits
  // `currentColor` from the existing button color cascade, so the
  // Pulse / Atelier / Console theme cycle keeps working unchanged.
  //
  // Wave 2.3 Lane Q.2 (G.1): collapse the sidebar to an icon-only rail
  // matching the bundle's `.rail`/`.rail-btn` chrome (`styles.css` lines
  // 197-217; bundle width 48px). Labels stay in the DOM for screen
  // readers + tests but become visually hidden; the existing `title`
  // attribute drives the on-hover tooltip. AppShell narrows the rail
  // column from 76px to 52px accordingly. No nav entries are dropped —
  // the bundle's `VIEWS` array still drives 13 top + 2 bottom (Account
  // / Help) per Chromaticity.html lines 57-71 / 176-177.
  import Icon, { type IconName } from './Icon.svelte';
  import { routeStore, type RouteName } from './stores/route.svelte';

  interface NavEntry {
    readonly route: RouteName;
    readonly label: string;
    readonly icon: IconName;
    readonly dividerBefore?: boolean;
  }

  interface DisabledEntry {
    readonly label: string;
    readonly icon: IconName;
    readonly section: 'top' | 'bottom';
    readonly dividerBefore?: boolean;
  }

  // Grouped visually into 4 semantic clusters via subtle horizontal dividers
  // (Live Stage | Create & Assets | Control & I/O | System) while preserving
  // canonical route ordering and full keyboard/a11y parity.
  const NAV_TOP: readonly (NavEntry | DisabledEntry)[] = [
    { route: 'perform', label: 'Perform', icon: 'perform' },
    { route: 'mix', label: 'Mix', icon: 'layers' },
    { route: 'library', label: 'Library', icon: 'library', dividerBefore: true },
    { route: 'author', label: 'Author', icon: 'edit' },
    { route: 'audio-uniforms', label: 'Audio', icon: 'audio' },
    { route: 'playlist', label: 'Playlist', icon: 'list' },
    { route: 'sources', label: 'Sources', icon: 'media' },
    { route: 'midi', label: 'MIDI', icon: 'midi', dividerBefore: true },
    { route: 'osc', label: 'OSC', icon: 'link' },
    { route: 'import', label: 'Import', icon: 'plus' },
    { route: 'patch', label: 'Patch', icon: 'cpu' },
    { route: 'settings', label: 'Settings', icon: 'settings', dividerBefore: true },
    { route: 'license', label: 'License', icon: 'lock' },
    { route: 'welcome', label: 'Welcome', icon: 'heart' },
  ] as const;

  // Wave 1d-α PR 1 promotes the Help bottom-section entry from a
  // disabled stub to a routed view (`views/Help.svelte`). The Account
  // entry remains disabled pending entitlement-tier UI work. The
  // bottom-section list now mixes wired + disabled entries; the
  // `isWired` discriminator below already covers both shapes.
  const NAV_BOTTOM: readonly (NavEntry | DisabledEntry)[] = [
    { route: 'help', label: 'Help', icon: 'help' },
  ] as const;

  function isWired(entry: NavEntry | DisabledEntry): entry is NavEntry {
    return 'route' in entry;
  }
</script>

<nav class="sidebar-rail" aria-label="Primary navigation">
  <ul class="sidebar-rail__section sidebar-rail__section--top">
    {#each NAV_TOP as entry (entry.label)}
      <li class:sidebar-rail__item--divider={entry.dividerBefore === true}>
        {#if isWired(entry)}
          <button
            type="button"
            class="sidebar-rail__btn"
            class:active={routeStore.current === entry.route}
            aria-current={routeStore.current === entry.route ? 'page' : undefined}
            title={entry.label}
            onclick={() => routeStore.set(entry.route)}
          >
            <span class="sidebar-rail__glyph">
              <Icon name={entry.icon} size={20} />
            </span>
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
            <span class="sidebar-rail__glyph">
              <Icon name={entry.icon} size={20} />
            </span>
            <span class="sidebar-rail__label">{entry.label}</span>
          </button>
        {/if}
      </li>
    {/each}
  </ul>
  <ul class="sidebar-rail__section sidebar-rail__section--bottom">
    {#each NAV_BOTTOM as entry (entry.label)}
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
            <span class="sidebar-rail__glyph">
              <Icon name={entry.icon} size={20} />
            </span>
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
            <span class="sidebar-rail__glyph">
              <Icon name={entry.icon} size={20} />
            </span>
            <span class="sidebar-rail__label">{entry.label}</span>
          </button>
        {/if}
      </li>
    {/each}
  </ul>
</nav>

<style>
  /* Token-driven scoped styles. No new tokens added — `--color-bg-1`
   * (sidebar surface), `--color-bg-3` (hover), `--color-primary` +
   * `--color-on-primary` (active) cover every state.
   *
   * Wave 2.3 Lane Q.2 (G.1) — icon-only rail mirroring the bundle's
   * `.rail` chrome (`styles.css` lines 207-217). The bundle rail is
   * 48 px wide with 36×36 buttons centred horizontally; we match
   * those dimensions and visually hide the `__label` element so
   * screen readers + existing widget tests (which query the label
   * text for ordering / count) keep working. The icon's `title`
   * attribute on the button still drives native tooltips on hover. */
  .sidebar-rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    background: var(--color-bg-1);
    border-right: 1px solid var(--color-line-2);
    padding: var(--space-sm) 0;
    overflow-y: auto;
    width: 100%;
  }
  .sidebar-rail__section {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    width: 100%;
  }
  .sidebar-rail__btn {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    background: transparent;
    color: var(--color-text-2);
    border: none;
    border-radius: var(--radius-sm);
    padding: 0;
    position: relative;
    cursor: pointer;
    transition:
      background 0.12s ease,
      color 0.12s ease;
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
  }
  /* Bundle-faithful accent strip on the active icon — `styles.css`
   * lines 213 `.rail-btn.active::before`. Carves a 2 px primary tick
   * down the left edge of the active button. */
  .sidebar-rail__btn.active::before {
    content: '';
    position: absolute;
    left: -6px;
    top: 6px;
    bottom: 6px;
    width: 2px;
    background: var(--color-primary);
    border-radius: 2px;
    box-shadow: 0 0 calc(8px * var(--glow, 1)) var(--color-primary);
  }
  .sidebar-rail__btn:disabled {
    color: var(--color-text-4);
    cursor: not-allowed;
  }
  .sidebar-rail__glyph {
    /* Icon inherits `currentColor` from the surrounding button text
     * color cascade. */
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 0;
    color: inherit;
  }
  /* G.1 — labels stay in the DOM (screen readers + Vitest suites
   * that query `.sidebar-rail__label` text) but are visually hidden
   * to match the bundle's icon-only rail. The button's native
   * `title` attribute provides the hover tooltip. */
  .sidebar-rail__label {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  .sidebar-rail__item--divider {
    position: relative;
    margin-top: 6px;
    padding-top: 7px;
  }
  .sidebar-rail__item--divider::before {
    content: '';
    position: absolute;
    top: 0;
    left: 10px;
    right: 10px;
    height: 1px;
    background: var(--color-line-2);
  }
</style>
