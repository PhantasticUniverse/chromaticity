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
    readonly groupRoutes?: readonly RouteName[];
    readonly dividerBefore?: boolean;
  }

  // Consolidated 6-workspace + 1-utility labeled rail:
  //   Live Stage: Perform, Mix
  //   Content & Create: Library (Scenes/Playlist/Sources/Import), Author (Editor/Audio/Patch)
  //   Control: I/O (MIDI/OSC)
  //   System: Settings (Preferences/License), Help
  const NAV_TOP: readonly NavEntry[] = [
    { route: 'perform', label: 'Perform', icon: 'perform' },
    { route: 'mix', label: 'Mix', icon: 'layers' },
    {
      route: 'library',
      label: 'Library',
      icon: 'library',
      groupRoutes: ['library', 'playlist', 'sources', 'import'],
      dividerBefore: true,
    },
    {
      route: 'author',
      label: 'Author',
      icon: 'edit',
      groupRoutes: ['author', 'audio-uniforms', 'patch'],
    },
    {
      route: 'midi',
      label: 'I/O',
      icon: 'midi',
      groupRoutes: ['midi', 'osc'],
      dividerBefore: true,
    },
    {
      route: 'settings',
      label: 'Settings',
      icon: 'settings',
      groupRoutes: ['settings', 'license'],
      dividerBefore: true,
    },
  ] as const;

  const NAV_BOTTOM: readonly NavEntry[] = [
    { route: 'help', label: 'Help', icon: 'help', groupRoutes: ['help', 'welcome'] },
  ] as const;

  function isEntryActive(entry: NavEntry): boolean {
    if (entry.groupRoutes) {
      return entry.groupRoutes.includes(routeStore.current);
    }
    return routeStore.current === entry.route;
  }
</script>

<nav class="sidebar-rail" aria-label="Primary navigation">
  <ul class="sidebar-rail__section sidebar-rail__section--top">
    {#each NAV_TOP as entry (entry.label)}
      <li class:sidebar-rail__item--divider={entry.dividerBefore === true}>
        <button
          type="button"
          class="sidebar-rail__btn"
          class:active={isEntryActive(entry)}
          aria-current={isEntryActive(entry) ? 'page' : undefined}
          title={entry.label}
          onclick={() => routeStore.set(entry.route)}
        >
          <span class="sidebar-rail__glyph">
            <Icon name={entry.icon} size={18} />
          </span>
          <span class="sidebar-rail__label">{entry.label}</span>
        </button>
      </li>
    {/each}
  </ul>
  <ul class="sidebar-rail__section sidebar-rail__section--bottom">
    {#each NAV_BOTTOM as entry (entry.label)}
      <li>
        <button
          type="button"
          class="sidebar-rail__btn"
          class:active={isEntryActive(entry)}
          aria-current={isEntryActive(entry) ? 'page' : undefined}
          title={entry.label}
          onclick={() => routeStore.set(entry.route)}
        >
          <span class="sidebar-rail__glyph">
            <Icon name={entry.icon} size={18} />
          </span>
          <span class="sidebar-rail__label">{entry.label}</span>
        </button>
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
    width: 54px;
    min-height: 44px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    background: transparent;
    color: var(--color-text-2);
    border: none;
    border-radius: var(--radius-sm);
    padding: 5px 2px;
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
  .sidebar-rail__btn.active::before {
    content: '';
    position: absolute;
    left: -5px;
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
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 0;
    color: inherit;
  }
  .sidebar-rail__label {
    font-family: var(--font-family-sans);
    font-size: 9.5px;
    font-weight: 500;
    letter-spacing: 0.02em;
    line-height: 1;
    color: inherit;
    white-space: nowrap;
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
