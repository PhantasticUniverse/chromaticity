<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
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
  // Wave-27 PR1 mounted `<SnapBar>` as Perform-gated app-level chrome
  // per the bundle (`Chromaticity.html` line 286, gated on
  // `view === "perform"`). The W39 follow-up corrects the W27 PR1 mount
  // position from "between TopBar and routed view" to "between routed
  // view and StatusBar" — the bundle places `<SnapBar>` AFTER `<main>`
  // and BEFORE `<StatusBar>`, and `DESIGN.md` lines 290-297 codify that
  // ordering in the layout block. The fix flips the inside-`__main`
  // ordering and converts `__main` from a single padded scroll
  // container into a flex column whose `__view` wrapper owns the
  // padding + overflow:auto so the snap bar can pin to the bottom of
  // the column without being shoved off-screen by the view's scroll.
  // Wave 2.1 Lane A.3 wires the SnapBar into the engine `recall_snap`
  // / `save_snap` / `clear_snap` IPC surface via `snapStore` — the
  // widget now owns its active-slot state from the store, so the
  // shell drops its previous local `snapActive` shadowing.
  //
  // Wave-28 PR1 mounts `<DegradedBanner>` directly under the TopBar and
  // above the routed view. Bundle adoption site: `Chromaticity.html`
  // line 263 — banner sits flush against the session HUD row. Content
  // is the bundle's "integrated GPU" warning fixture; real
  // adapter-health probing is `EV-INTEGRATION-MATRIX` scope (same
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
  // `routeStore` import coupling. The remaining dismiss-on-click no-op
  // rows (Snap recall, OSC address surfacing, support-bundle
  // generation, offline-docs dispatch) stay fixture-only per the
  // `EV-INTEGRATION-MATRIX` synthetic narrowing — the row is `Pass` at
  // synthetic-coverage scope, with operator-facing wiring deferred
  // post-MVP.
  //
  // Wave-44 mounts `<CrashResumeBanner>` directly under the
  // `<DegradedBanner>` and above the routed view, matching the
  // bundle's `Chromaticity.html` adoption site (lines 271-280). Lane
  // 4 FIX PASS wires the banner to the shell's real
  // `get_crash_resume_status` command: setup restores the supported
  // runtime classes before scheduler ticks start, records a one-shot
  // status payload, and this shell shows the banner only when that
  // payload is present. `onSupport` remains a no-op pending the
  // support-bundle modal port; `onViewLog` routes to Settings.
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
  import type { CrashResumeStatusPayload } from './ipc/bindings';
  import SidebarRail from './SidebarRail.svelte';
  import { overlayStack } from './state';
  import StatusBar from './StatusBar.svelte';
  import { capabilitiesStore } from './stores/capabilities.svelte';
  import { liveFxStore } from './stores/liveFx.svelte';
  import { loadedSceneStore } from './stores/loadedScene.svelte';
  import { midiBindingsStore } from './stores/midiBindings.svelte';
  import { playlistStore } from './stores/playlist.svelte';
  import { routeStore } from './stores/route.svelte';
  import TopBar from './TopBar.svelte';
  import AudioUniforms from './views/AudioUniforms.svelte';
  import Author from './views/Author.svelte';
  import Help from './views/Help.svelte';
  import Import from './views/Import.svelte';
  import Library from './views/Library.svelte';
  import License from './views/License.svelte';
  import MIDI from './views/MIDI.svelte';
  import Mix from './views/Mix.svelte';
  import Osc from './views/Osc.svelte';
  import Patch from './views/Patch.svelte';
  import Perform from './views/Perform.svelte';
  import Playlist from './views/Playlist.svelte';
  import Settings from './views/Settings.svelte';
  import Sources from './views/Sources.svelte';
  import Welcome from './views/Welcome.svelte';
  import {
    CommandPalette,
    CrashResumeBanner,
    DEFAULT_COMMAND_PALETTE_ACTIONS,
    DegradedBanner,
    SceneCanvas,
    SnapBar,
    type CommandPaletteAction,
    WatermarkOverlay,
  } from './widgets';

  let mixVisited = $state(false);
  let stageDockOpen = $state(false);
  const showStageDockToggle = $derived(
    routeStore.current === 'library' ||
      routeStore.current === 'midi' ||
      routeStore.current === 'osc' ||
      routeStore.current === 'playlist' ||
      routeStore.current === 'sources' ||
      routeStore.current === 'patch' ||
      routeStore.current === 'settings',
  );
  $effect(() => {
    if (routeStore.current === 'mix') {
      mixVisited = true;
    }
  });

  // Wave 2.3 Lane Q.2 (G.2) — DegradedBanner default visibility flips
  // from true to false so the unsigned local dev build no longer
  // surfaces the synthetic "Running on integrated GPU" warning. The
  // bundle gates the same banner on the dev-only `tweaks.emptyStates`
  // flag (`Chromaticity.html` lines 261-269), which defaults to false
  // in `TWEAK_DEFAULTS` (line 54). Real adapter-health probing remains
  // `EV-INTEGRATION-MATRIX` scope; once that lands the visibility
  // will read from a capability flag rather than a hardcoded default.
  type CrashResumeBannerState =
    | { readonly kind: 'hidden' }
    | { readonly kind: 'visible'; readonly payload: CrashResumeStatusPayload };

  let bannerVisible = $state(false);
  let crashResume = $state<CrashResumeBannerState>({ kind: 'hidden' });
  const commandPaletteOpen = $derived(overlayStack.isOpen('command-palette'));

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
    void capabilitiesStore.refreshFromEngine();
    if (midiBindingsStore.bindings.length > 0) {
      void midiBindingsStore.startListening();
    }
    void (async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const status = await invoke<CrashResumeStatusPayload | null>('get_crash_resume_status');
        if (status) {
          crashResume = { kind: 'visible', payload: status };
        }
      } catch {
        // Browser-mode / Vitest: no Tauri runtime, no recovered
        // snapshot, and therefore no banner.
      }
    })();
  });

  $effect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey) {
        if (event.key === 'k' || event.key === 'K') {
          event.preventDefault();
          overlayStack.toggle('command-palette');
        }
      }
    };
    window.addEventListener('keydown', handler);
    return () => {
      window.removeEventListener('keydown', handler);
      overlayStack.close('command-palette');
    };
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
    {#if crashResume.kind === 'visible'}
      <div class="app-shell__banner">
        <CrashResumeBanner
          showName={crashResume.payload.showName}
          controls={crashResume.payload.controls}
          snaps={crashResume.payload.snaps}
          playlist={crashResume.payload.playlist}
          adapters={crashResume.payload.adapters}
          scene={crashResume.payload.scene}
          p50={crashResume.payload.p50}
          onViewLog={() => routeStore.set('settings')}
          onSupport={() => {
            /* W44 follow-up: support-bundle modal port */
          }}
          onDismiss={() => (crashResume = { kind: 'hidden' })}
        />
      </div>
    {/if}
    <div
      class="app-shell__view"
      class:app-shell__view--perform={routeStore.current === 'perform' ||
        routeStore.current === 'mix' ||
        routeStore.current === 'author' ||
        routeStore.current === 'patch'}
    >
      {#if mixVisited}
        <div
          class="app-shell__mix-keepalive"
          hidden={routeStore.current !== 'mix'}
          aria-hidden={routeStore.current !== 'mix' ? 'true' : undefined}
        >
          <Mix />
        </div>
      {/if}
      {#if routeStore.current === 'perform'}
        <Perform />
      {:else if routeStore.current === 'mix'}
        <!-- Rendered inside keepalive wrapper above so live Twitch/Webcam/Media never drop on route switch -->
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
      {:else if routeStore.current === 'help'}
        <Help />
      {/if}
    </div>
    {#if routeStore.current === 'perform'}
      <div class="app-shell__snapbar">
        <SnapBar />
      </div>
    {/if}
    {#if showStageDockToggle}
      <aside class="app-shell__stage-dock" aria-label="Live stage monitor">
        {#if stageDockOpen}
          <div class="stage-dock__card">
            <header class="stage-dock__header">
              <span class="stage-dock__title">
                LIVE STAGE · {loadedSceneStore.snapshot?.name ?? 'Active Output'}
              </span>
              <button
                type="button"
                class="stage-dock__close"
                onclick={() => (stageDockOpen = false)}
                aria-label="Collapse live stage monitor"
              >
                ×
              </button>
            </header>
            <div class="stage-dock__canvas">
              {#if liveFxStore.blackoutActive}
                <div class="stage-dock__blackout">BLACKOUT</div>
              {:else}
                <SceneCanvas kind="rings" seed={4} />
              {/if}
            </div>
            <footer class="stage-dock__controls">
              <button type="button" onclick={() => void playlistStore.previous()}>Prev</button>
              <button type="button" onclick={() => void playlistStore.next()}>Next</button>
              <button
                type="button"
                class="stage-dock__perform"
                onclick={() => routeStore.set('perform')}
              >
                Perform
              </button>
            </footer>
          </div>
        {:else}
          <button
            type="button"
            class="stage-dock__pill"
            data-testid="toggle-stage-dock"
            onclick={() => (stageDockOpen = true)}
          >
            Live Stage
          </button>
        {/if}
      </aside>
    {/if}
  </main>
  <footer class="app-shell__status">
    <StatusBar />
  </footer>
</div>

<ConsoleDrawer />

<CommandPalette
  open={commandPaletteOpen}
  onClose={() => overlayStack.close('command-palette')}
  commands={commandPaletteActions}
/>

<WatermarkOverlay visible={watermarkVisible} />

<style>
  /* Wave 2.3 Lane Q.2 (G.1) — sidebar column narrows from 76 px to
   * 52 px so the icon-only `<SidebarRail>` lines up with the bundle's
   * 48 px rail (`Chromaticity.html` lines 168-179 + `styles.css` line
   * 197 `.app{grid-template-columns:48px 1fr;}`). We keep a 52 px
   * column to honour the W12 16-px icon-target + breathing room
   * around the 36 px button hit-area; reducing further crowds the
   * focus-ring outline against the right border. */
  .app-shell {
    display: grid;
    grid-template-columns: 52px 1fr;
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
    overflow: visible;
    position: relative;
    z-index: 40;
  }
  .app-shell__rail {
    grid-area: rail;
    overflow: hidden;
  }
  /* W39 follow-up: `__main` is now a flex column so the snap bar can
   * pin to the bottom of the main column (per bundle Chromaticity.html
   * line 286 ordering and DESIGN.md layout block). The `__view`
   * wrapper carries the padding + overflow that `__main` used to own
   * directly, leaving `__main` free of padding so the banner (top)
   * and snap bar (bottom) sit flush against the column edges without
   * the negative-margin trick the W27 PR1 / W28 PR1 layout needed. */
  .app-shell__main {
    grid-area: main;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }
  .app-shell__view {
    flex: 1;
    overflow: auto;
    padding: var(--space-lg) var(--space-xl);
    min-height: 0;
  }
  .app-shell__view--perform {
    padding: var(--space-md);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .app-shell__mix-keepalive {
    width: 100%;
    height: 100%;
    min-height: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .app-shell__mix-keepalive[hidden] {
    display: none;
  }
  .app-shell__banner {
    flex-shrink: 0;
  }
  .app-shell__snapbar {
    flex-shrink: 0;
  }
  .app-shell__status {
    grid-area: status;
    overflow: hidden;
  }
  .app-shell__stage-dock {
    position: fixed;
    right: 16px;
    bottom: 40px;
    z-index: 40;
  }
  .stage-dock__pill {
    padding: 5px 12px;
    background: var(--color-bg-2);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-full);
    font-family: var(--font-family-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.45);
  }
  .stage-dock__pill:hover {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }
  .stage-dock__card {
    width: 264px;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.6);
  }
  .stage-dock__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: var(--color-bg-2);
    border-bottom: 1px solid var(--color-line-1);
    font-family: var(--font-family-mono);
    font-size: 10px;
    color: var(--color-text-1);
  }
  .stage-dock__title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .stage-dock__close {
    background: transparent;
    border: none;
    color: var(--color-text-2);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
  }
  .stage-dock__canvas {
    position: relative;
    height: 148px;
    background: #06070d;
  }
  .stage-dock__blackout {
    height: 100%;
    display: grid;
    place-items: center;
    font-family: var(--font-family-mono);
    font-size: 11px;
    letter-spacing: 0.12em;
    color: var(--color-danger);
  }
  .stage-dock__controls {
    display: flex;
    gap: 6px;
    padding: 6px 8px;
    background: var(--color-bg-2);
    border-top: 1px solid var(--color-line-1);
  }
  .stage-dock__controls button {
    flex: 1;
    padding: 4px 6px;
    font-family: var(--font-family-mono);
    font-size: 11px;
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    cursor: pointer;
  }
  .stage-dock__perform {
    color: var(--color-primary);
    border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-line-2));
  }
</style>
