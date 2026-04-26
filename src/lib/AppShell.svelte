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
  // map. With four routes the ladder is more readable, and Svelte 5's
  // exhaustiveness via the `RouteName` literal union catches a missing
  // branch at `npm run check` time. Per CLAUDE.md "three similar lines
  // beats a premature abstraction" — this is exactly that case.
  //
  // The `<HotkeyOverlay>` modal rides above every view at z-index 1000;
  // its keydown handler is owned by `App.svelte` so the global `?`
  // listener stays installed regardless of active route.
  import SidebarRail from './SidebarRail.svelte';
  import StatusBar from './StatusBar.svelte';
  import { routeStore } from './stores/route.svelte';
  import Author from './views/Author.svelte';
  import Library from './views/Library.svelte';
  import Perform from './views/Perform.svelte';
  import Playlist from './views/Playlist.svelte';
  import Settings from './views/Settings.svelte';
</script>

<div class="app-shell">
  <aside class="app-shell__rail">
    <SidebarRail />
  </aside>
  <main class="app-shell__main">
    {#if routeStore.current === 'perform'}
      <Perform />
    {:else if routeStore.current === 'library'}
      <Library />
    {:else if routeStore.current === 'author'}
      <Author />
    {:else if routeStore.current === 'playlist'}
      <Playlist />
    {:else if routeStore.current === 'settings'}
      <Settings />
    {/if}
  </main>
  <footer class="app-shell__status">
    <StatusBar />
  </footer>
</div>

<style>
  .app-shell {
    display: grid;
    grid-template-columns: 76px 1fr;
    grid-template-rows: 1fr 28px;
    grid-template-areas:
      'rail main'
      'status status';
    height: 100vh;
    width: 100vw;
    background: var(--color-bg-0);
    color: var(--color-text-0);
    font-family: var(--font-family-sans);
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
  .app-shell__status {
    grid-area: status;
    overflow: hidden;
  }
</style>
