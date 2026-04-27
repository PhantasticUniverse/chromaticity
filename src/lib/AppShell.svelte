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
  import ConsoleDrawer from './ConsoleDrawer.svelte';
  import SidebarRail from './SidebarRail.svelte';
  import StatusBar from './StatusBar.svelte';
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
  import { SnapBar } from './widgets';

  let snapActive = $state(2);
</script>

<div class="app-shell">
  <header class="app-shell__topbar">
    <TopBar />
  </header>
  <aside class="app-shell__rail">
    <SidebarRail />
  </aside>
  <main class="app-shell__main">
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
  .app-shell__status {
    grid-area: status;
    overflow: hidden;
  }
</style>
