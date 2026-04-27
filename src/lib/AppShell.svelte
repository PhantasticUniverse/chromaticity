<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-12 PR3 application shell. CSS-Grid layout matching the wireframe
  // captures (`.playwright-mcp/wireframe-reference/01-…07-…png`):
  //
  //   ┌────────┬────────────────────────────────┐
  //   │        │                                │
  //   │ rail   │  active view                   │
  //   │ (12    │                                │
  //   │  icons)│                                │
  //   │        │                                │
  //   ├────────┴────────────────────────────────┤
  //   │ status bar                              │
  //   └─────────────────────────────────────────┘
  //
  // Route dispatch is a `{#if}` ladder rather than a route → component
  // map. With ten routes the ladder is still more readable than a
  // dispatch table, and Svelte 5's exhaustiveness via the `RouteName`
  // literal union catches a missing branch at `npm run check` time.
  // Per CLAUDE.md "three similar lines beats a premature abstraction" —
  // this is still exactly that case, even at 10 branches.
  //
  // The `<HotkeyOverlay>` modal rides above every view at z-index 1000;
  // its keydown handler is owned by `App.svelte` so the global `?`
  // listener stays installed regardless of active route.
  //
  // Wave-16 PR2 adds the four enabled-but-stub views (Welcome / Sources
  // / Audio Uniforms / License) per the v0.2 sidebar redesign. Each
  // stub renders an `EmptyState`; W17+ replaces each with a full port
  // of its respective bundle JSX.
  //
  // Wave-18 PR3 promotes the MIDI route from the previously-disabled
  // sidebar stub to a full bundle port (`views/MIDI.svelte`). Dispatch
  // arm added in bundle ordering (after `sources`, before `osc`).
  //
  // Wave-19 PR1 promotes the Patch route from the previously-disabled
  // sidebar stub to a full bundle port (`views/Patch.svelte`). Dispatch
  // arm added in bundle ordering (after `osc`, before `settings`).
  //
  // Wave-19 PR2 promotes the Import route from the previously-disabled
  // sidebar stub to a full bundle port (`views/Import.svelte`). Dispatch
  // arm added in bundle ordering (between `osc` and `patch`). After this
  // PR every routed entry in the v0.2 sidebar's top section is mounted.
  //
  // Wave-21 PR4 mounts `<ConsoleDrawer>` as an overlay layer above the
  // grid. The drawer's visibility is owned by `consoleStore.isOpen`
  // (toggled by the StatusBar's Console button). The component renders
  // nothing when closed, so the cost of always mounting it is a single
  // top-level `{#if}` evaluation per reactive tick.
  //
  // Wave-27 PR1 mounts `<SnapBar>` between the TopBar and the routed
  // view, gated on `routeStore.current === 'perform'`. The bundle
  // (`Chromaticity.html` line 286) treats SnapBar as app-level chrome
  // visible only on the Perform surface; the local `snapActive` state
  // tracks which slot is highlighted. Frozen-frame: the deferred wiring
  // onto the engine `SnapSlotId` IPC surface is `EV-INTEGRATION-MATRIX`
  // scope (the eight perform-time bookmarks here are a different domain
  // from the sixteen engine slots in `src/lib/snap.ts`).
  //
  // Wave-28 PR1 mounts `<DegradedBanner>` directly under the TopBar and
  // above the SnapBar / routed view. Bundle adoption site:
  // `Chromaticity.html` line 263 — banner sits flush against the session
  // HUD row. Content is the bundle's "integrated GPU" warning fixture;
  // real adapter-health probing is `EV-INTEGRATION-MATRIX` scope (same
  // posture as the W14 stub status pill in `TopBar.svelte`). `onAction`
  // routes to Settings via `routeStore.set('settings')`; `onDismiss`
  // hides the banner via local `bannerVisible` state.
  //
  // Wave-29 PR1 mounts `<CommandPalette>` as a fixed overlay at
  // z-index 1010 (one layer above the HotkeyOverlay's 1000 in
  // `App.svelte`). A window-level keydown handler toggles
  // `commandPaletteOpen` on Cmd+K (macOS) / Ctrl+K (Windows / Linux);
  // `event.preventDefault()` suppresses the browser default. The two
  // routable rows (`launch-scene` → Library, `open-hotkeys` → Settings)
  // attach their `onSelect` here so the widget fixture stays free of
  // `routeStore` import coupling. The remaining five rows are
  // dismiss-on-click no-ops pending Phase-3+/4a `EV-INTEGRATION-MATRIX`
  // wiring (Snap recall, OSC address surfacing, support-bundle
  // generation, offline-docs dispatch).
  //
  // Wave-33a mounts `<WatermarkOverlay>` as a fixed bottom-right
  // overlay at z-index 990 (under HotkeyOverlay / CommandPalette).
  // Wave-33b threads `capabilitiesStore.has('render.watermark.off')`
  // through and inverts it (visible when the capability is denied),
  // closing the W33a hardcoded-fixture posture. Per `adr/ADR-009-
  // activation-and-entitlements.md` the operative MVP watermark is
  // drawn by the Rust rendering backend on the scene canvas; this UI
  // overlay is the bundle's dev-time visual mock of the no-license
  // state, threaded the way `BUNDLE-PROVENANCE-v0.2-beta.md` line 32
  // describes ("WatermarkOverlay threaded into Perform live Preview
  // via ChromaCtx.licensed").
  import ConsoleDrawer from './ConsoleDrawer.svelte';
  import SidebarRail from './SidebarRail.svelte';
  import StatusBar from './StatusBar.svelte';
  import { capabilitiesStore } from './stores/capabilities.svelte';
  import { routeStore } from './stores/route.svelte';
  import TopBar from './TopBar.svelte';
  import AudioUniforms from './views/AudioUniforms.svelte';
  import Author from './views/Author.svelte';
  import Import from './views/Import.svelte';
  import Library from './views/Library.svelte';
  import License from './views/License.svelte';
  import MIDI from './views/MIDI.svelte';
  import Osc from './views/Osc.svelte';
  import Patch from './views/Patch.svelte';
  import Perform from './views/Perform.svelte';
  import Playlist from './views/Playlist.svelte';
  import Settings from './views/Settings.svelte';
  import Sources from './views/Sources.svelte';
  import Welcome from './views/Welcome.svelte';
  import {
    CommandPalette,
    DEFAULT_COMMAND_PALETTE_ACTIONS,
    DegradedBanner,
    SnapBar,
    type CommandPaletteAction,
    WatermarkOverlay,
  } from './widgets';

  let snapActive = $state(2);
  let bannerVisible = $state(true);
  let commandPaletteOpen = $state(false);

  // Watermark visibility is the inverse of the `render.watermark.off`
  // capability per ADR-009: present when the capability is absent
  // (default / expired / tampered token), absent when held. The W33b
  // capabilities-store seeds from the `LICENSE_FIXTURE` `activated`
  // matrix (which grants `render.watermark.off`), so this `$derived`
  // resolves to `false` in the live activated-license render path.
  // To exercise the overlay manually, flip the seed in
  // `src/lib/stores/capabilities.svelte.ts` or call
  // `capabilitiesStore.setSnapshot({...})` from devtools.
  const watermarkVisible = $derived(!capabilitiesStore.has('render.watermark.off'));

  const BANNER_FIXTURE = {
    severity: 'warning' as const,
    message: 'Running on integrated GPU. Performance reduced.',
    actionLabel: 'Diagnostics',
  };

  const COMMAND_PALETTE_ROUTING: Record<string, () => void> = {
    'launch-scene': () => routeStore.set('library'),
    'open-hotkeys': () => routeStore.set('settings'),
  };

  const commandPaletteActions: readonly CommandPaletteAction[] =
    DEFAULT_COMMAND_PALETTE_ACTIONS.map((action) => {
      const onSelect = COMMAND_PALETTE_ROUTING[action.id];
      return onSelect ? { ...action, onSelect } : action;
    });

  $effect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey) {
        if (event.key === 'k' || event.key === 'K') {
          event.preventDefault();
          commandPaletteOpen = !commandPaletteOpen;
        }
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  });
</script>

<div class="app-shell">
  <header class="app-shell__topbar">
    <TopBar />
  </header>
  <aside class="app-shell__rail">
    <SidebarRail />
  </aside>
  <main class="app-shell__main">
    {#if bannerVisible}
      <div class="app-shell__banner">
        <DegradedBanner
          severity={BANNER_FIXTURE.severity}
          message={BANNER_FIXTURE.message}
          actionLabel={BANNER_FIXTURE.actionLabel}
          onAction={() => routeStore.set('settings')}
          onDismiss={() => (bannerVisible = false)}
        />
      </div>
    {/if}
    {#if routeStore.current === 'perform'}
      <div class="app-shell__snapbar">
        <SnapBar active={snapActive} onPick={(slot) => (snapActive = slot)} />
      </div>
      <Perform />
    {:else if routeStore.current === 'library'}
      <Library />
    {:else if routeStore.current === 'author'}
      <Author />
    {:else if routeStore.current === 'audio-uniforms'}
      <AudioUniforms />
    {:else if routeStore.current === 'playlist'}
      <Playlist />
    {:else if routeStore.current === 'sources'}
      <Sources />
    {:else if routeStore.current === 'midi'}
      <MIDI />
    {:else if routeStore.current === 'osc'}
      <Osc />
    {:else if routeStore.current === 'import'}
      <Import />
    {:else if routeStore.current === 'patch'}
      <Patch />
    {:else if routeStore.current === 'settings'}
      <Settings />
    {:else if routeStore.current === 'license'}
      <License />
    {:else if routeStore.current === 'welcome'}
      <Welcome />
    {/if}
  </main>
  <footer class="app-shell__status">
    <StatusBar />
  </footer>
</div>

<ConsoleDrawer />

<CommandPalette
  open={commandPaletteOpen}
  onClose={() => (commandPaletteOpen = false)}
  commands={commandPaletteActions}
/>

<WatermarkOverlay visible={watermarkVisible} />

<style>
  .app-shell {
    display: grid;
    grid-template-columns: 76px 1fr;
    grid-template-rows: 52px 1fr 28px;
    grid-template-areas:
      'topbar topbar'
      'rail main'
      'status status';
    height: 100vh;
    width: 100vw;
    background: var(--color-bg-0);
    color: var(--color-text-0);
    font-family: var(--font-family-sans);
  }
  .app-shell__topbar {
    grid-area: topbar;
    overflow: hidden;
  }
  .app-shell__rail {
    grid-area: rail;
    overflow: hidden;
  }
  .app-shell__main {
    grid-area: main;
    overflow: auto;
    padding: var(--space-3xl) var(--space-4xl);
  }
  /* Break the SnapBar out of `__main`'s side+top padding so it spans
   * the full main column width directly under the TopBar, matching the
   * bundle's `Chromaticity.html` layout where SnapBar sits flush against
   * the TopBar / view boundary. The bottom margin restores the gap to
   * the routed view container. */
  .app-shell__snapbar {
    margin: calc(-1 * var(--space-3xl)) calc(-1 * var(--space-4xl)) var(--space-3xl);
  }
  /* Same full-bleed treatment for the W28 PR1 DegradedBanner. When the
   * banner renders alongside the SnapBar (on Perform), the SnapBar
   * sibling re-applies its own negative top margin so the two strips
   * stack flush. When only the banner renders, the bottom margin opens
   * the standard view gap. */
  .app-shell__banner {
    margin: calc(-1 * var(--space-3xl)) calc(-1 * var(--space-4xl)) var(--space-3xl);
  }
  .app-shell__banner + .app-shell__snapbar {
    margin-top: 0;
  }
  .app-shell__status {
    grid-area: status;
    overflow: hidden;
  }
</style>
