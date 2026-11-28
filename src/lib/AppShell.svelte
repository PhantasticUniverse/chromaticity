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
  import { SETLIST_FIXTURE } from './data/setlist-fixtures';
  import { setControl } from './ipc';
  import { setAudioThreshold } from './ipc/audio';
  import type { ControlId, CrashResumeStatusPayload } from './ipc/bindings';
  import SidebarRail from './SidebarRail.svelte';
  import { overlayStack } from './state';
  import StatusBar from './StatusBar.svelte';
  import { audioSnapshotStore } from './stores/audioSnapshot.svelte';
  import { bpmControlStore } from './stores/bpmControl.svelte';
  import { capabilitiesStore } from './stores/capabilities.svelte';
  import { djAutopilotStore } from './stores/djAutopilot.svelte';
  import { liveFxStore } from './stores/liveFx.svelte';
  import { loadedSceneStore } from './stores/loadedScene.svelte';
  import { midiBindingsStore } from './stores/midiBindings.svelte';
  import { playlistStore } from './stores/playlist.svelte';
  import { routeStore, type RouteName } from './stores/route.svelte';
  import { snapStore } from './stores/snap.svelte';
  import { stageDockStore } from './stores/stageDock.svelte';
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
  import { sceneKindFromMetadata, sceneSeedFromMetadata } from './widgets/scene-kind-from-metadata';

  const META_STROBE_CONTROL_ID = 1070 as ControlId;
  const META_BLACKOUT_CONTROL_ID = 1071 as ControlId;

  interface WorkspaceSubTab {
    readonly route: RouteName;
    readonly label: string;
  }

  const WORKSPACE_SUBTABS: readonly {
    readonly routes: readonly RouteName[];
    readonly tabs: readonly WorkspaceSubTab[];
  }[] = [
    {
      routes: ['library', 'playlist', 'sources', 'import'],
      tabs: [
        { route: 'library', label: 'Scenes' },
        { route: 'playlist', label: 'Playlist' },
        { route: 'sources', label: 'Sources' },
        { route: 'import', label: 'Import' },
      ],
    },
    {
      routes: ['author', 'audio-uniforms', 'patch'],
      tabs: [
        { route: 'author', label: 'Editor' },
        { route: 'audio-uniforms', label: 'Audio Uniforms' },
        { route: 'patch', label: 'Patch' },
      ],
    },
    {
      routes: ['midi', 'osc'],
      tabs: [
        { route: 'midi', label: 'MIDI' },
        { route: 'osc', label: 'OSC' },
      ],
    },
    {
      routes: ['settings', 'license'],
      tabs: [
        { route: 'settings', label: 'Preferences' },
        { route: 'license', label: 'License' },
      ],
    },
  ];

  const activeSubTabs = $derived(
    WORKSPACE_SUBTABS.find((group) => group.routes.includes(routeStore.current))?.tabs ?? null,
  );

  let mixVisited = $state(false);
  const showStageDockToggle = $derived(stageDockStore.isEligibleRoute(routeStore.current));
  const stageDockVisible = $derived(showStageDockToggle && stageDockStore.open);
  const stageDockKind = $derived(
    sceneKindFromMetadata({
      id: loadedSceneStore.snapshot?.id ?? 'glitch',
      name: loadedSceneStore.snapshot?.name ?? 'Glitch',
    }),
  );
  const stageDockSeed = $derived(
    sceneSeedFromMetadata({
      id: loadedSceneStore.snapshot?.id ?? 'glitch',
      name: loadedSceneStore.snapshot?.name ?? 'Glitch',
    }),
  );
  const stageDockBpmLabel = $derived.by(() => {
    if (bpmControlStore.isOff) return 'BPM OFF';
    const liveBpm =
      audioSnapshotStore.snapshot?.bpm_availability === 'Ready'
        ? audioSnapshotStore.snapshot.bpm
        : null;
    const resolved = bpmControlStore.isListening
      ? (liveBpm ?? 120)
      : (bpmControlStore.manualBpm ?? 120);
    return `BPM ${resolved.toFixed(1)}`;
  });
  $effect(() => {
    if (routeStore.current === 'mix') {
      mixVisited = true;
    }
  });
  // Real DJ Autopilot phrase timer — advances the playlist every N bars at the effective BPM.
  $effect(() => {
    if (!djAutopilotStore.isActive) return;
    const liveBpm =
      audioSnapshotStore.snapshot?.bpm_availability === 'Ready'
        ? audioSnapshotStore.snapshot.bpm
        : null;
    const resolved = bpmControlStore.isListening
      ? (liveBpm ?? 120)
      : (bpmControlStore.manualBpm ?? 120);
    const bpm = Math.max(40, resolved);
    const intervalMs = Math.round(djAutopilotStore.phraseIntervalBars * 4 * (60_000 / bpm));
    const id = window.setInterval(() => {
      void playlistStore.next();
    }, intervalMs);
    return () => window.clearInterval(id);
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
    'snap-recall': () => {
      routeStore.set('perform');
      void snapStore.recall(snapStore.activeSlot ?? 1);
    },
    'snap-save': () => {
      routeStore.set('perform');
      void snapStore.save(snapStore.activeSlot ?? 1);
    },
    'osc-address': () => routeStore.set('osc'),
    'open-hotkeys': () => overlayStack.open('hotkeys'),
    'support-bundle': () => routeStore.set('settings'),
    'offline-docs': () => routeStore.set('help'),
  };

  const WORKSPACE_PALETTE_ACTIONS: readonly CommandPaletteAction[] = [
    {
      id: 'nav-perform',
      label: 'Workspace: Perform',
      meta: 'live deck',
      onSelect: () => routeStore.set('perform'),
    },
    {
      id: 'nav-mix',
      label: 'Workspace: Mix',
      meta: 'compositor & stream',
      onSelect: () => routeStore.set('mix'),
    },
    {
      id: 'nav-library',
      label: 'Workspace: Library · Scenes',
      meta: '⌘L',
      onSelect: () => routeStore.set('library'),
    },
    {
      id: 'nav-playlist',
      label: 'Workspace: Library · Playlist',
      meta: 'cue sheet',
      onSelect: () => routeStore.set('playlist'),
    },
    {
      id: 'nav-sources',
      label: 'Workspace: Library · Sources',
      meta: 'media & live inputs',
      onSelect: () => routeStore.set('sources'),
    },
    {
      id: 'nav-import',
      label: 'Workspace: Library · Import',
      meta: 'shadertoy / synScene',
      onSelect: () => routeStore.set('import'),
    },
    {
      id: 'nav-author',
      label: 'Workspace: Author · Editor',
      meta: 'GLSL IDE',
      onSelect: () => routeStore.set('author'),
    },
    {
      id: 'nav-audio',
      label: 'Workspace: Author · Audio Uniforms',
      meta: 'FFT & BPM',
      onSelect: () => routeStore.set('audio-uniforms'),
    },
    {
      id: 'nav-patch',
      label: 'Workspace: Author · Patch',
      meta: 'node routing',
      onSelect: () => routeStore.set('patch'),
    },
    {
      id: 'nav-midi',
      label: 'Workspace: I/O · MIDI',
      meta: 'controllers & learn',
      onSelect: () => routeStore.set('midi'),
    },
    {
      id: 'nav-osc',
      label: 'Workspace: I/O · OSC',
      meta: '⌘O · routes',
      onSelect: () => routeStore.set('osc'),
    },
    {
      id: 'nav-settings',
      label: 'Workspace: Settings · Preferences',
      meta: '⌘,',
      onSelect: () => routeStore.set('settings'),
    },
    {
      id: 'nav-license',
      label: 'Workspace: Settings · License',
      meta: 'entitlements',
      onSelect: () => routeStore.set('license'),
    },
    {
      id: 'nav-help',
      label: 'Workspace: Help',
      meta: '⌘/ · offline docs',
      onSelect: () => routeStore.set('help'),
    },
    {
      id: 'fx-blackout',
      label: 'Live FX: Toggle Blackout',
      meta: 'B · panic',
      onSelect: () => {
        const active = liveFxStore.toggleBlackout();
        void setControl(META_BLACKOUT_CONTROL_ID, active ? 1 : 0).catch(() => {});
      },
    },
    {
      id: 'toggle-stage-dock',
      label: 'View: Toggle Stage Monitor Dock',
      meta: 'STAGE',
      onSelect: () => stageDockStore.toggle(),
    },
  ];

  const SCENE_PALETTE_ACTIONS: readonly CommandPaletteAction[] = SETLIST_FIXTURE.map((scene) => ({
    id: `scene-${scene.idx}`,
    label: `Scene: ${scene.name}`,
    meta: `perform · ${scene.tag}`,
    onSelect: () => {
      routeStore.set('perform');
    },
  }));

  const commandPaletteActions: readonly CommandPaletteAction[] = [
    ...DEFAULT_COMMAND_PALETTE_ACTIONS.map((action) => {
      const onSelect = COMMAND_PALETTE_ROUTING[action.id];
      return onSelect ? { ...action, onSelect } : action;
    }),
    ...WORKSPACE_PALETTE_ACTIONS,
    ...SCENE_PALETTE_ACTIONS,
  ];

  function isEditableTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
    if (target.isContentEditable) return true;
    return target.closest('.cm-editor') !== null;
  }

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
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey) {
        if (event.key === 'k' || event.key === 'K') {
          event.preventDefault();
          overlayStack.toggle('command-palette');
          return;
        }
        if (event.key === ',') {
          event.preventDefault();
          routeStore.set('settings');
          return;
        }
        if (event.key === 'l' || event.key === 'L') {
          event.preventDefault();
          routeStore.set('library');
          return;
        }
        if (event.key === 'o' || event.key === 'O') {
          event.preventDefault();
          routeStore.set('osc');
          return;
        }
        if (event.key === '/') {
          event.preventDefault();
          routeStore.set('help');
          return;
        }
      }
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (isEditableTarget(event.target)) return;
      if (overlayStack.isOpen('command-palette') || overlayStack.isOpen('hotkeys')) return;

      if (event.key === 'b' || event.key === 'B') {
        if (!event.repeat) {
          event.preventDefault();
          const active = liveFxStore.toggleBlackout();
          void setControl(META_BLACKOUT_CONTROL_ID, active ? 1 : 0).catch(() => {});
        }
        return;
      }
      if (event.key === 'T' && event.shiftKey) {
        // Shift+T: set the audio noise gate to the current input level.
        if (!event.repeat) {
          event.preventDefault();
          void setAudioThreshold(audioSnapshotStore.snapshot?.rms ?? 0);
        }
        return;
      }
      if (event.key === 's' || event.key === 'S') {
        if (!event.repeat) {
          event.preventDefault();
          if (liveFxStore.activateStrobe()) {
            void setControl(META_STROBE_CONTROL_ID, 1).catch(() => {});
          }
        }
        return;
      }
      if (routeStore.current === 'perform' && /^[1-8]$/.test(event.key)) {
        event.preventDefault();
        void snapStore.recall(Number(event.key));
        return;
      }
      if (routeStore.current === 'perform' || routeStore.current === 'playlist') {
        if (event.key === ' ') {
          event.preventDefault();
          playlistStore.togglePlayback();
          return;
        }
        if (event.key === 'ArrowRight') {
          event.preventDefault();
          void playlistStore.next();
          return;
        }
        if (event.key === 'ArrowLeft') {
          event.preventDefault();
          void playlistStore.previous();
          return;
        }
      }
    };
    const releaseStrobeSafe = () => {
      if (liveFxStore.releaseStrobe()) {
        void setControl(META_STROBE_CONTROL_ID, 0).catch(() => {});
      }
    };
    const onKeyUp = (event: KeyboardEvent) => {
      if (event.key === 's' || event.key === 'S') {
        releaseStrobeSafe();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', onKeyUp);
    window.addEventListener('blur', releaseStrobeSafe);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('keyup', onKeyUp);
      window.removeEventListener('blur', releaseStrobeSafe);
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
    {#if activeSubTabs}
      <nav class="app-shell__subtabs" aria-label="Workspace sections">
        {#each activeSubTabs as tab (tab.route)}
          <button
            type="button"
            class="app-shell__subtab"
            class:active={routeStore.current === tab.route}
            aria-current={routeStore.current === tab.route ? 'page' : undefined}
            onclick={() => routeStore.set(tab.route)}
          >
            {tab.label}
          </button>
        {/each}
      </nav>
    {/if}
    <div class="app-shell__workspace" class:app-shell__workspace--docked={stageDockVisible}>
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
      {#if stageDockVisible}
        <aside class="stage-dock" aria-label="Live stage monitor">
          <header class="stage-dock__header">
            <span class="stage-dock__title">
              STAGE · {loadedSceneStore.snapshot?.name ?? 'Glitch'}
            </span>
            <button
              type="button"
              class="stage-dock__close"
              onclick={() => stageDockStore.setOpen(false)}
              aria-label="Collapse live stage monitor"
            >
              ×
            </button>
          </header>
          <div class="stage-dock__canvas">
            {#if liveFxStore.blackoutActive}
              <div class="stage-dock__blackout">BLACKOUT</div>
            {:else}
              <SceneCanvas kind={stageDockKind} seed={stageDockSeed} />
            {/if}
            {#if liveFxStore.strobeActive}
              <span class="stage-dock__badge stage-dock__badge--strobe">STROBE</span>
            {/if}
          </div>
          <div class="stage-dock__meta mono">
            <span>{stageDockBpmLabel}</span>
            <span
              class="stage-dock__state"
              class:stage-dock__state--danger={liveFxStore.blackoutActive}
            >
              {liveFxStore.blackoutActive ? 'BLACKOUT' : 'MONITOR'}
            </span>
          </div>
          <div class="stage-dock__fx">
            <button
              type="button"
              class="stage-dock__fx-btn"
              class:stage-dock__fx-btn--danger={liveFxStore.blackoutActive}
              aria-pressed={liveFxStore.blackoutActive ? 'true' : 'false'}
              onclick={() => {
                const active = liveFxStore.toggleBlackout();
                void setControl(META_BLACKOUT_CONTROL_ID, active ? 1 : 0).catch(() => {});
              }}
            >
              {liveFxStore.blackoutActive ? 'BLACKOUT ON' : 'Blackout (B)'}
            </button>
            <button
              type="button"
              class="stage-dock__fx-btn"
              aria-pressed={playlistStore.playback === 'playing' ? 'true' : 'false'}
              onclick={() => playlistStore.togglePlayback()}
            >
              {playlistStore.playback === 'playing' ? 'Pause' : 'Play'}
            </button>
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
        </aside>
      {/if}
    </div>
    {#if routeStore.current === 'perform'}
      <div class="app-shell__snapbar">
        <SnapBar />
      </div>
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
    grid-template-columns: 64px 1fr;
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
  .app-shell__subtabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 var(--space-lg);
    height: 32px;
    flex-shrink: 0;
    background: var(--color-bg-1);
    border-bottom: 1px solid var(--color-line-1);
  }
  .app-shell__subtab {
    height: 100%;
    padding: 0 var(--space-md);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    transition:
      color 0.12s ease,
      border-color 0.12s ease;
  }
  .app-shell__subtab:hover {
    color: var(--color-text-0);
  }
  .app-shell__subtab.active {
    color: var(--color-text-0);
    border-bottom-color: var(--color-primary);
  }
  .app-shell__workspace {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-height: 0;
    overflow: hidden;
  }
  .app-shell__workspace--docked {
    grid-template-columns: minmax(0, 1fr) 260px;
  }
  .app-shell__view {
    overflow: auto;
    padding: var(--space-lg) var(--space-xl);
    min-height: 0;
    min-width: 0;
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
  .stage-dock {
    display: flex;
    flex-direction: column;
    background: var(--color-bg-1);
    border-left: 1px solid var(--color-line-2);
    min-height: 0;
    overflow: hidden;
  }
  .stage-dock__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
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
    height: 164px;
    background: #06070d;
    border-bottom: 1px solid var(--color-line-1);
  }
  .stage-dock__badge {
    position: absolute;
    top: 8px;
    right: 8px;
    padding: 2px 6px;
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    background: var(--color-danger);
    color: #0b0c10;
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
  .stage-dock__meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    font-size: 11px;
    color: var(--color-text-2);
    border-bottom: 1px solid var(--color-line-1);
  }
  .stage-dock__state {
    color: var(--color-text-1);
  }
  .stage-dock__state--danger {
    color: var(--color-danger);
  }
  .stage-dock__fx {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    padding: 8px 10px 0;
  }
  .stage-dock__fx-btn {
    padding: 5px 8px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: 10px;
    cursor: pointer;
  }
  .stage-dock__fx-btn:hover {
    color: var(--color-text-0);
    border-color: var(--color-line-1);
  }
  .stage-dock__fx-btn--danger {
    background: var(--color-danger);
    border-color: var(--color-danger);
    color: #0b0c10;
    font-weight: 700;
  }
  .stage-dock__controls {
    display: flex;
    gap: 6px;
    padding: 8px 10px;
    background: var(--color-bg-2);
    border-bottom: 1px solid var(--color-line-1);
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
