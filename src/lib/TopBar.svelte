<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-15 PR1 top bar / session HUD primitive — visible across every
  // route per `.playwright-mcp/wireframe-reference/01..07-*.png`. The
  // wireframes' top-right session HUD (adapter status pills + FPS +
  // CPU% + REC + GO LIVE) is consistent across all 7 views; this
  // primitive owns that chrome and mounts above the AppShell main
  // content area.
  //
  // Layout: 3-column flex.
  //   - LEFT  : Chromaticity wordmark + view-specific subtitle
  //   - CENTER: spacer (future: scene-timecode / GO-LIVE countdown)
  //   - RIGHT : adapter status pills + FPS + GO LIVE button
  //
  // W15 scope (locked):
  //   - Static wordmark + per-route subtitle.
  //   - Adapter pills render with placeholder labels (OUT / DISP 2 /
  //     LFX / MIDI / REC). No live wiring; W14 PR4 OSC view already
  //     surfaces real adapter health via osc_health_snapshot, but
  //     MIDI / DISP / SYPHON / SPOUT / NDI integration is W15+ work.
  //   - FPS readout is `—` placeholder. Real telemetry wires when
  //     the rendering backend (ADR-011) lands and exposes a frame
  //     counter via runtime IPC.
  //   - GO LIVE button is visual-only — no `livePressed` state, no
  //     IPC dispatch. Click is currently a no-op. Real "go live"
  //     semantics (output activation + entitlement check) are
  //     post-MVP per ADR-009.
  //
  // Per `feedback_wireframes_normative_for_ui_ux.md`, this is the
  // wireframe-faithful chrome that anchors operator orientation
  // across views.
  //
  // W20 view-internal icon sweep: GO LIVE button glyph picks up the W19
  // PR3 vendored `Icon.svelte` primitive (`play` size 11) per
  // `Chromaticity.html`'s `<button className="btn go sm" aria-label="Go
  // live"><Icon name="play" size={11} /> GO LIVE</button>` mapping.
  //
  // Wave-34a swaps the W15 PR1 inline `top-bar__pill` placeholder for
  // the bundle-faithful `<AdapterPill>` widget (`v02_supplement.jsx`
  // lines 20-44; bundle adoption at `Chromaticity.html` lines 228-230).
  //
  // Wave 2.4 Lane PERF.D replaces the previous hard-coded
  // `STATUS_PILLS` fixture with live store reads. Each of the three
  // pills now reflects real cross-seam state:
  //
  //   - **OUT pill** — derived from `loadedSceneStore.snapshot` (a
  //     scene is loaded ⇒ the engine could output if the render
  //     backend were live). Real Syphon / Spout / NDI / Display
  //     wiring is a Wave 1c-β follow-up; until that lands the OUT
  //     pill is a UI proxy.
  //   - **MIDI pill** — derived from `midiStore.devices` length;
  //     the store hydrates on TopBar mount and re-polls on focus
  //     events; this is the same enumeration that powers the MIDI
  //     view Devices tab (Wave 2.1 D.1).
  //   - **AUDIO pill** — derived from `audioSnapshotStore.snapshot`;
  //     `snapshot.degraded` maps to the red disconnected state so
  //     operator chrome surfaces recovered last-known audio state.
  //   - **OSC pill** — driven by the new `oscHealthStore` which polls
  //     the `osc_health_snapshot` Tauri command every 5 s (Wave 2.1
  //     D.3 shipped the command + bridge type).
  import Icon from './Icon.svelte';
  import { closeOutputWindow, openOutputWindow } from './ipc/output';
  import { getRecorderStatus, startRecording, stopRecording } from './ipc/recorder';
  import { fetchGpuRenderStatus } from './ipc/render';
  import { audioSnapshotStore } from './stores/audioSnapshot.svelte';
  import { bpmControlStore } from './stores/bpmControl.svelte';
  import { djAutopilotStore } from './stores/djAutopilot.svelte';
  import { loadedSceneStore } from './stores/loadedScene.svelte';
  import { midiStore } from './stores/midiStore.svelte';
  import { oscHealthStore } from './stores/oscHealth.svelte';
  import { performVariantStore, type PerformVariant } from './stores/performVariant.svelte';
  import { routeStore, type RouteName } from './stores/route.svelte';
  import { settingsStore } from './stores/settings.svelte';
  import { stageDockStore } from './stores/stageDock.svelte';
  import { transportStore } from './stores/transport.svelte';
  import { AdapterPill, BeatDots, VuStereo, type AdapterPillState } from './widgets';

  // Wave 2.1 Lane B.1 — STANDARD / CONSOLE segmented toggle. Only
  // visible on the Perform route; click handlers dispatch into the
  // module-singleton `performVariantStore` so the Perform view re-
  // renders the matching layout. Mapping is fixed: STANDARD → classic
  // (canonical wireframe); CONSOLE → console (3-column dense, the
  // bundle's "dense" variant).
  const VARIANT_TOGGLES: ReadonlyArray<{ readonly id: PerformVariant; readonly label: string }> = [
    { id: 'classic', label: 'STANDARD' },
    { id: 'console', label: 'CONSOLE' },
  ] as const;

  // Wave 2.1 Lane B.2 — top-bar transport HUD elements (timecode, beat
  // dots, BPM, audio meter, camera button, record button, GPU readout).
  // Per the canonical bundle (`Chromaticity.html` lines 191-257) and
  // the AO smoke-test direction the HUD chrome only shows on the
  // Perform route — the other routes show their view-specific meta
  // instead (a future enhancement; today other routes simply omit the
  // HUD cluster).
  //
  // Default BPM fallback when no audio snapshot is present matches the
  // bundle's `BeatDots bpm={128}` (`Chromaticity.html` line 205) and
  // the AudioReactivity widget's 120-default elsewhere — we use 120 as
  // the safer pre-IPC value since the bundle's 128 is a fixture-show
  // tempo.
  const DEFAULT_BPM = 120;

  // Subscribe to the live audio snapshot store so BPM + VU meter read
  // engine data when available. No-op outside Tauri (vite dev / Vitest)
  // where the snapshot stays `null` and consumers fall back to
  // defaults / synthetic generators.
  $effect(() => {
    void audioSnapshotStore.subscribe();
  });

  // Auto-start the transport clock on TopBar mount and halt it on
  // unmount. The TopBar lives for the entire app lifetime so this is
  // effectively "clock runs whenever the app is open", but we honour
  // the lifecycle contract so test unmounts (and any future TopBar
  // remount path) don't leak rAF handles. start() is idempotent.
  $effect(() => {
    transportStore.start();
    return () => {
      transportStore.stop();
    };
  });

  // Reactive timecode + BPM readouts. `$derived` re-runs whenever
  // `transportStore.elapsedMs` or `audioSnapshotStore.snapshot` change,
  // so the HUD chips repaint on every clock tick / IPC event.
  const currentTimecode = $derived.by(() => {
    // Touch elapsedMs so $derived tracks reactivity through the method
    // call (transportStore.timecode() reads state internally).
    void transportStore.elapsedMs;
    return transportStore.timecode();
  });

  const audioSnapshot = $derived(audioSnapshotStore.snapshot);
  const audioBpmReady = $derived(audioSnapshot?.bpm_availability === 'Ready');
  const detectedBpm = $derived(audioBpmReady ? (audioSnapshot?.bpm ?? DEFAULT_BPM) : DEFAULT_BPM);
  const currentBpm = $derived(bpmControlStore.effectiveBpm(detectedBpm));
  const bpmMode = $derived(bpmControlStore.mode);
  const bpmListening = $derived(bpmControlStore.isListening);
  const bpmOff = $derived(bpmControlStore.isOff);
  const bpmLocked = $derived(bpmMode === 'manual');
  const bpmModeLabel = $derived(
    bpmMode === 'auto' ? 'AUTO' : bpmMode === 'manual' ? 'LOCK' : 'OFF',
  );
  // BPM display formatting: bundle renders one decimal place
  // (`Chromaticity.html` line 208 `128.0`). In manual LOCK mode we
  // always show the locked tempo even while audio detection warms up.
  // In OFF mode we show `OFF`.
  const bpmDisplay = $derived(
    bpmOff ? 'OFF' : bpmListening && audioSnapshot && !audioBpmReady ? '--' : currentBpm.toFixed(1),
  );

  // Beatmatch & BPM popover state.
  let beatmatchOpen = $state(false);
  let bpmAnchorEl = $state<HTMLDivElement | null>(null);
  let bpmTriggerEl = $state<HTMLButtonElement | null>(null);

  /** Format phase offset for human display (Unicode minus, signed beats). */
  function formatPhaseDisplay(beats: number): string {
    const rounded = Math.round(beats * 100) / 100;
    if (rounded === 0) return '0.00 beats';
    const abs = Math.abs(rounded).toFixed(2);
    return rounded < 0 ? `\u2212${abs} beats` : `+${abs} beats`;
  }

  const phaseDisplay = $derived(formatPhaseDisplay(bpmControlStore.phaseOffsetBeats));
  /** Machine-serialized ASCII numeric strings for DOM input `.value` attributes. */
  const editableBpmBase = $derived(bpmOff ? detectedBpm : currentBpm);
  const manualBpmInputValue = $derived(String(editableBpmBase));
  const phaseScrubInputValue = $derived(String(bpmControlStore.phaseOffsetBeats));

  function toggleBeatmatchPopover(): void {
    beatmatchOpen = !beatmatchOpen;
  }

  function closeBeatmatchPopover(restoreFocus = false): void {
    beatmatchOpen = false;
    if (restoreFocus) {
      bpmTriggerEl?.focus();
    }
  }

  function handleSelectAutoMode(): void {
    bpmControlStore.setAuto();
  }

  function handleSelectManualMode(): void {
    if (bpmControlStore.mode !== 'manual') {
      bpmControlStore.setManualBpm(detectedBpm);
    }
  }

  function handleSelectOffMode(): void {
    bpmControlStore.setOff();
  }

  function handleManualBpmInput(event: Event): void {
    const target = event.currentTarget as HTMLInputElement;
    const parsed = Number(target.value);
    if (Number.isFinite(parsed)) {
      bpmControlStore.setManualBpm(parsed);
    }
  }

  function handlePhaseScrubInput(event: Event): void {
    const target = event.currentTarget as HTMLInputElement;
    const parsed = Number(target.value);
    if (Number.isFinite(parsed)) {
      bpmControlStore.setPhaseOffset(parsed);
    }
  }

  // Close the beatmatch popover on Escape or outside pointer click.
  $effect(() => {
    if (!beatmatchOpen) return;
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        event.preventDefault();
        closeBeatmatchPopover(true);
      }
    };
    const onMouseDown = (event: MouseEvent): void => {
      const target = event.target as Node | null;
      if (target && bpmAnchorEl && !bpmAnchorEl.contains(target)) {
        closeBeatmatchPopover(false);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    document.addEventListener('mousedown', onMouseDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      document.removeEventListener('mousedown', onMouseDown);
    };
  });

  // Automatically close the popover when navigating away from Perform.
  $effect(() => {
    if (routeStore.current !== 'perform' && beatmatchOpen) {
      beatmatchOpen = false;
    }
  });

  // Per-route subtitle. Hardcoded for W15; future enhancement could
  // accept a per-view dynamic count via route-store metadata (e.g.
  // Library passes "1,247 scenes / 12 authors" once a real corpus
  // populates).
  const ROUTE_TITLES: Readonly<Record<RouteName, string>> = {
    perform: 'Perform',
    mix: 'Mix',
    library: 'Library',
    author: 'Author · Editor',
    'audio-uniforms': 'Author · Audio Uniforms',
    playlist: 'Library · Playlist',
    sources: 'Library · Sources',
    midi: 'I/O · MIDI',
    osc: 'I/O · OSC',
    import: 'Library · Import',
    patch: 'Author · Patch',
    settings: 'Settings · Preferences',
    license: 'Settings · License',
    welcome: 'Welcome',
    help: 'Help',
  };

  // The OSC subtitle interpolates the live inbound port from
  // settingsStore (default 9000, NOT the historical 7000 placeholder),
  // mirroring the OSC view body (`Osc.svelte` `listenPort`). Reading the
  // settings-backed value keeps the top bar from drifting away from the
  // port the OSC adapter actually binds on launch.
  const oscListenPort = $derived(settingsStore.snapshot.osc.port);
  const routeSubtitle = $derived(
    routeStore.current === 'osc'
      ? oscHealthStore.snapshot?.state === 'Healthy'
        ? `OSC · LISTENING — Port ${oscListenPort}`
        : `OSC · PREVIEW — Port ${oscListenPort}`
      : ROUTE_TITLES[routeStore.current],
  );

  // Adapter status pills mirroring the v0.2 wireframe pack's top-right
  // session HUD chrome (consistent across welcome / library / perform /
  // scene-editor / playlist / sources / midi / osc / audio-uniforms /
  // import / settings / license / patch). Three status pills (OUT /
  // MIDI / OSC) + separate REC indicator + FPS + CPU + GO LIVE button.
  //
  // The OUT pill is a roll-up of all output adapters (Syphon / Spout /
  // NDI / Display) per the v0.2 abstraction; per-adapter detail goes
  // into Settings → Diagnostics. Wave 2.4 PERF.D uses the loaded-scene
  // store as the pre-alpha proxy for "engine has a scene loaded and
  // could output if the render backend were live" — real Syphon /
  // NDI wiring is a Wave 1c-β follow-up.
  //
  // The MIDI pill reflects the live device enumeration powering the
  // MIDI view Devices tab (Wave 2.1 D.1). At least one enumerated
  // device flips the pill to `ok`; otherwise the pill renders `off`.
  //
  // The OSC pill reads from `oscHealthStore`, polling the
  // `osc_health_snapshot` Tauri command every 5 s. Mapping:
  //   - `Healthy`    → `ok`
  //   - `Recovering` → `warming`
  //   - `Degraded`   → `disconnected`
  //   - `null` (no snapshot yet / poll failed) → `off`

  // Subscribe to MIDI enumeration on TopBar mount so the pill reflects
  // the current device count from app launch. `refresh()` is on
  // demand (pull-model) and swallows the `invoke` rejection that
  // fires outside Tauri.
  $effect(() => {
    void midiStore.refresh();
    // Hydrate settings so the OSC subtitle reflects a non-default
    // configured port; `load()` swallows the non-Tauri rejection and is
    // idempotent (same call the OSC + Settings views make).
    void settingsStore.load();
  });

  // Start the OSC health polling loop on TopBar mount; halt on
  // unmount. `start()` is idempotent so re-mounts don't leak timers.
  $effect(() => {
    oscHealthStore.start();
    return () => {
      oscHealthStore.stop();
    };
  });

  /**
   * Map the W4 `OscHealthState`-shaped state vocabulary onto
   * AdapterPill's bundle vocabulary. The bundle has six states
   * (`off / warming / ok / rate-limited / disconnected / blocked`);
   * the OSC adapter today only emits three of those semantics. A
   * `null` snapshot (no poll yet, or poll failed under no-Tauri
   * context) collapses to `off`. When the OSC adapter learns to
   * distinguish rate-limit + blocked states, this mapping widens at
   * the source rather than here.
   */
  function mapOscHealthToPillState(
    state: 'Healthy' | 'Recovering' | 'Degraded' | null,
  ): AdapterPillState {
    switch (state) {
      case 'Healthy':
        return 'ok';
      case 'Recovering':
        return 'warming';
      case 'Degraded':
        return 'disconnected';
      case null:
        return 'off';
    }
  }

  /** Stage output window open state toggled by the GO LIVE button. */
  let outputWindowOpen = $state(false);

  /** OUT pill state — `ok` only when live stage output window is actually open. */
  const outState: AdapterPillState = $derived(outputWindowOpen ? 'ok' : 'off');

  /** MIDI pill state — `ok` when at least one enumerated MIDI device is present. */
  const midiState: AdapterPillState = $derived(midiStore.devices.length > 0 ? 'ok' : 'off');

  /** AUDIO pill state — degraded snapshots surface as red/offline. */
  const audioState: AdapterPillState = $derived.by(() => {
    const snap = audioSnapshotStore.snapshot;
    if (!snap) return 'off';
    if (snap.degraded) return 'disconnected';
    if (snap.bpm_availability !== 'Ready' || snap.fft_availability !== 'Ready') return 'warming';
    return 'ok';
  });

  /** OSC pill state — derived from the polling store snapshot. */
  const oscState: AdapterPillState = $derived(
    mapOscHealthToPillState(oscHealthStore.snapshot?.state ?? null),
  );

  /** OUT pill tooltip detail — reflects live stage output state. */
  const outDetail = $derived(
    outputWindowOpen
      ? `Stage output live · ${loadedSceneStore.snapshot?.name ?? 'Active Scene'}`
      : 'Stage output offline — click to configure Graphics',
  );

  /** MIDI pill tooltip detail — device count summary. */
  const midiDetail = $derived(
    midiStore.devices.length > 0
      ? `${midiStore.devices.length} device${midiStore.devices.length === 1 ? '' : 's'}`
      : 'No MIDI devices',
  );

  /** AUDIO pill tooltip detail — readiness / degraded summary. */
  const audioDetail = $derived.by(() => {
    const snap = audioSnapshotStore.snapshot;
    if (!snap) return 'Audio snapshot offline';
    if (snap.degraded) return 'Audio degraded · recovered last-known snapshot';
    if (snap.bpm_availability !== 'Ready' || snap.fft_availability !== 'Ready') {
      return `Warming · BPM ${snap.bpm_availability} · FFT ${snap.fft_availability}`;
    }
    return 'Audio snapshot ready';
  });

  /** OSC pill tooltip detail — local address when bound; state word otherwise. */
  const oscDetail = $derived.by(() => {
    const snap = oscHealthStore.snapshot;
    if (!snap) return 'OSC adapter offline';
    if (snap.local_addr) return `Listening · ${snap.local_addr}`;
    return `Unbound · ${snap.attempted_binds} attempts`;
  });
  /** Live crash-safe fMP4 recording state backed by `app/shell` recorder IPC. */
  let isRecording = $state(false);
  let recordingBusy = $state(false);

  /** Reactive 1-click DJ Autopilot active state. */
  const djAutopilotActive = $derived(djAutopilotStore.isActive);

  // Hydrate initial recorder status on TopBar mount (gracefully no-ops outside Tauri).
  $effect(() => {
    let active = true;
    void getRecorderStatus().then((status) => {
      if (active && status) {
        isRecording = status.is_recording;
      }
    });
    return () => {
      active = false;
    };
  });

  async function handleRecordToggle(): Promise<void> {
    if (recordingBusy) return;
    recordingBusy = true;
    try {
      if (isRecording) {
        const status = await stopRecording();
        isRecording = status ? status.is_recording : false;
      } else {
        const status = await startRecording();
        isRecording = status ? status.is_recording : false;
      }
      if (typeof window !== 'undefined') {
        window.dispatchEvent(
          new CustomEvent('chromaticity:recording-change', { detail: { isRecording } }),
        );
      }
    } finally {
      recordingBusy = false;
    }
  }

  /** Live GPU FPS & frame-budget utilization polled from `gpu_render_status`. */
  let liveFps = $state<number | null>(null);
  let liveGpuPct = $state<number | null>(null);

  $effect(() => {
    let active = true;
    const pollGpu = async (): Promise<void> => {
      const status = await fetchGpuRenderStatus();
      if (!active || !status || status.state !== 'active' || status.last_frame_ms <= 0) {
        return;
      }
      const ms = status.last_frame_ms;
      liveFps = Math.min(240, 1000 / ms);
      liveGpuPct = Math.min(100, Math.max(1, Math.round((ms / 16.6667) * 100)));
    };
    void pollGpu();
    const timer = window.setInterval(() => void pollGpu(), 1000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  });

  const fpsDisplay = $derived(liveFps !== null ? liveFps.toFixed(1) : '—');
  const gpuDisplay = $derived(liveGpuPct !== null ? `${liveGpuPct}%` : '—');
  const isWelcomeRoute = $derived(routeStore.current === 'welcome');
  const canToggleStageDock = $derived(stageDockStore.isEligibleRoute(routeStore.current));

  let goLiveArmed = $state(false);
  let goLiveArmTimer: ReturnType<typeof setTimeout> | null = null;

  function armGoLiveGuard(): void {
    goLiveArmed = true;
    if (goLiveArmTimer !== null) clearTimeout(goLiveArmTimer);
    goLiveArmTimer = setTimeout(() => {
      goLiveArmed = false;
      goLiveArmTimer = null;
    }, 3000);
  }

  function clearGoLiveGuard(): void {
    goLiveArmed = false;
    if (goLiveArmTimer !== null) {
      clearTimeout(goLiveArmTimer);
      goLiveArmTimer = null;
    }
  }

  async function handleGoLiveClick(): Promise<void> {
    try {
      if (!goLiveArmed) {
        armGoLiveGuard();
        return;
      }
      clearGoLiveGuard();
      if (outputWindowOpen) {
        const status = await closeOutputWindow();
        outputWindowOpen = status.open;
      } else {
        const status = await openOutputWindow({ fullscreen: false });
        outputWindowOpen = status.open;
      }
    } catch {
      // Non-fatal in headless / browser preview mode.
    }
  }
</script>

<header class="top-bar">
  <div class="top-bar__brand">
    <span class="top-bar__brand-mark" aria-hidden="true"></span>
    <span class="top-bar__brand-text">
      <span class="top-bar__wordmark">Chromaticity</span>
      <span class="top-bar__title">{routeSubtitle}</span>
    </span>
  </div>

  <div class="top-bar__center">
    {#if routeStore.current === 'perform'}
      <!-- Wave 2.1 Lane B.1 — Perform variant toggle. Mirrors the
           bundle's STANDARD / CONSOLE chrome (`Chromaticity.html`
           lines ~74-88 + perform.jsx variant prop). Hidden on every
           non-Perform route per the bundle. -->
      <div class="top-bar__variant" role="group" aria-label="Perform layout variant">
        {#each VARIANT_TOGGLES as toggle (toggle.id)}
          <button
            type="button"
            class="top-bar__variant-btn"
            class:top-bar__variant-btn--active={performVariantStore.current === toggle.id}
            aria-pressed={performVariantStore.current === toggle.id ? 'true' : 'false'}
            onclick={() => performVariantStore.set(toggle.id)}
          >
            {toggle.label}
          </button>
        {/each}
      </div>

      <!-- Wave 2.1 Lane B.2 — transport HUD cluster (timecode + beat dots
           + BPM display + stereo VU meter). Mirrors the bundle's
           `Chromaticity.html` lines 203-222 mount inside the
           `showSessionHud` branch (Perform / Scene / Audio in the
           bundle; Perform-only here per Lane B.2 brief direction). The
           transport clock is driven by `transportStore`; BPM + VU read
           the live audio snapshot store with documented fallbacks. -->
      <span
        class="top-bar__transport-timecode"
        data-testid="top-bar-timecode"
        title="Session timecode"
      >
        <span class="top-bar__transport-timecode-text" aria-label="Session timecode">
          {currentTimecode}
        </span>
      </span>
      <span class="top-bar__transport-beat" data-testid="top-bar-beat-dots">
        <BeatDots bpm={currentBpm} />
      </span>
      <div class="top-bar__bpm-anchor" bind:this={bpmAnchorEl}>
        <button
          type="button"
          class="top-bar__transport-bpm"
          class:top-bar__transport-bpm--locked={bpmLocked}
          class:top-bar__transport-bpm--off={bpmOff}
          class:top-bar__transport-bpm--open={beatmatchOpen}
          data-testid="top-bar-bpm"
          aria-expanded={beatmatchOpen ? 'true' : 'false'}
          aria-controls="top-bar-beatmatch-popover"
          aria-haspopup="dialog"
          aria-label={bpmOff
            ? 'BPM Off. Open beatmatch controls'
            : `BPM ${bpmDisplay}, ${bpmListening ? 'Auto Listen' : 'Manual Lock'} mode. Open beatmatch controls`}
          title="BPM & Beatmatch controls"
          bind:this={bpmTriggerEl}
          onclick={toggleBeatmatchPopover}
        >
          <span class="top-bar__transport-bpm-key">BPM</span>
          {#if !bpmOff}
            <span
              class="top-bar__transport-bpm-value"
              class:top-bar__transport-bpm-value--warming={bpmListening &&
                audioSnapshot !== null &&
                !audioBpmReady}
            >
              {bpmDisplay}
            </span>
          {/if}
          <span
            class="top-bar__transport-bpm-mode"
            class:top-bar__transport-bpm-mode--lock={bpmLocked}
            class:top-bar__transport-bpm-mode--off={bpmOff}
            data-testid="top-bar-bpm-mode"
          >
            {bpmModeLabel}
          </span>
        </button>

        {#if beatmatchOpen}
          <div
            id="top-bar-beatmatch-popover"
            class="top-bar__beatmatch-popover"
            role="dialog"
            aria-label="Beatmatch and BPM controls"
            data-testid="top-bar-beatmatch-popover"
          >
            <div class="beatmatch__header">
              <span class="beatmatch__title">Beatmatch &amp; BPM</span>
              <span
                class="beatmatch__status"
                class:beatmatch__status--lock={bpmLocked}
                class:beatmatch__status--off={bpmOff}
              >
                {bpmOff ? 'OFF' : `${bpmDisplay} · ${bpmModeLabel}`}
              </span>
            </div>

            <div class="beatmatch__section">
              <span class="beatmatch__label">BPM Listening</span>
              <div class="beatmatch__segmented" role="group" aria-label="BPM tracking mode">
                <button
                  type="button"
                  class="beatmatch__seg-btn"
                  class:beatmatch__seg-btn--active={bpmListening}
                  aria-pressed={bpmListening ? 'true' : 'false'}
                  data-testid="bpm-mode-auto"
                  onclick={handleSelectAutoMode}
                >
                  Auto Listen
                </button>
                <button
                  type="button"
                  class="beatmatch__seg-btn"
                  class:beatmatch__seg-btn--active={bpmLocked}
                  aria-pressed={bpmLocked ? 'true' : 'false'}
                  data-testid="bpm-mode-lock"
                  onclick={handleSelectManualMode}
                >
                  Manual Lock
                </button>
                <button
                  type="button"
                  class="beatmatch__seg-btn"
                  class:beatmatch__seg-btn--active={bpmOff}
                  aria-pressed={bpmOff ? 'true' : 'false'}
                  data-testid="bpm-mode-off"
                  onclick={handleSelectOffMode}
                >
                  Off
                </button>
              </div>
            </div>

            <div class="beatmatch__section">
              <label class="beatmatch__label" for="top-bar-manual-bpm">Tempo (BPM)</label>
              <div class="beatmatch__tempo-row">
                <input
                  id="top-bar-manual-bpm"
                  type="number"
                  class="beatmatch__bpm-input"
                  min="40"
                  max="240"
                  step="0.1"
                  value={manualBpmInputValue}
                  data-testid="bpm-manual-input"
                  aria-label="Manual BPM"
                  oninput={handleManualBpmInput}
                />
                <button
                  type="button"
                  class="beatmatch__btn"
                  data-testid="bpm-half"
                  aria-label="Halve BPM"
                  onclick={() => bpmControlStore.setManualBpm(editableBpmBase / 2)}
                >
                  ÷2
                </button>
                <button
                  type="button"
                  class="beatmatch__btn"
                  data-testid="bpm-double"
                  aria-label="Double BPM"
                  onclick={() => bpmControlStore.setManualBpm(editableBpmBase * 2)}
                >
                  ×2
                </button>
                <button
                  type="button"
                  class="beatmatch__btn beatmatch__btn--accent"
                  data-testid="bpm-tap"
                  aria-label="Tap tempo"
                  onclick={() => bpmControlStore.tap()}
                >
                  TAP
                </button>
              </div>
            </div>

            <div class="beatmatch__section">
              <div class="beatmatch__phase-head">
                <label class="beatmatch__label" for="top-bar-phase-scrub">Phase Scrub</label>
                <span class="beatmatch__phase-value" data-testid="bpm-phase-display">
                  {phaseDisplay}
                </span>
              </div>
              <input
                id="top-bar-phase-scrub"
                type="range"
                class="beatmatch__slider"
                min="-0.5"
                max="0.5"
                step="0.01"
                value={phaseScrubInputValue}
                data-testid="bpm-phase-scrub"
                aria-label="Beatmatch phase scrub in beats"
                oninput={handlePhaseScrubInput}
              />
              <div class="beatmatch__slider-ticks" aria-hidden="true">
                <span>−0.5</span>
                <span>0</span>
                <span>+0.5</span>
              </div>
              <div class="beatmatch__nudge-row">
                <button
                  type="button"
                  class="beatmatch__btn"
                  data-testid="bpm-nudge-back"
                  aria-label="Nudge phase earlier by 5 percent of a beat"
                  onclick={() => bpmControlStore.nudgePhase(-0.05)}
                >
                  ‹ −5%
                </button>
                <button
                  type="button"
                  class="beatmatch__btn beatmatch__btn--resync"
                  data-testid="bpm-resync"
                  aria-label="Resync downbeat to now"
                  onclick={() => bpmControlStore.resyncDownbeat()}
                >
                  Resync 1
                </button>
                <button
                  type="button"
                  class="beatmatch__btn"
                  data-testid="bpm-nudge-forward"
                  aria-label="Nudge phase later by 5 percent of a beat"
                  onclick={() => bpmControlStore.nudgePhase(0.05)}
                >
                  +5% ›
                </button>
              </div>
            </div>
          </div>
        {/if}
      </div>
      <span class="top-bar__transport-vu" data-testid="top-bar-vu" aria-label="Audio meter">
        <VuStereo width={60} />
      </span>
    {/if}
  </div>

  <div class="top-bar__hud">
    {#if !isWelcomeRoute}
      <div class="top-bar__pills" role="group" aria-label="Adapter status">
        <AdapterPill
          kind="OUT"
          state={outState}
          detail={outDetail}
          onclick={() => routeStore.set('settings')}
        />
        <AdapterPill
          kind="AUDIO"
          state={audioState}
          detail={audioDetail}
          onclick={() => routeStore.set('settings')}
        />
        <AdapterPill
          kind="MIDI"
          state={midiState}
          detail={midiDetail}
          onclick={() => routeStore.set('midi')}
        />
        <AdapterPill
          kind="OSC"
          state={oscState}
          detail={oscDetail}
          onclick={() => routeStore.set('osc')}
        />
      </div>
      <span
        class="top-bar__metric"
        title={liveFps !== null
          ? 'Live GPU frames per second (from cached wgpu pipeline)'
          : 'Frames per second (no live render loop)'}
      >
        <span class="top-bar__metric-unit">fps</span>
        <span class="top-bar__metric-value">{fpsDisplay}</span>
      </span>
      <span
        class="top-bar__metric"
        title={liveGpuPct !== null
          ? 'Live GPU 60 Hz frame-time budget utilization'
          : 'GPU utilization (no live telemetry)'}
      >
        <span class="top-bar__metric-unit">gpu</span>
        <span class="top-bar__metric-value" data-testid="top-bar-gpu-value">{gpuDisplay}</span>
      </span>
    {/if}
    {#if canToggleStageDock}
      <button
        type="button"
        class="top-bar__stage-btn"
        class:top-bar__stage-btn--active={stageDockStore.open}
        data-testid="toggle-stage-dock"
        aria-pressed={stageDockStore.open ? 'true' : 'false'}
        title={stageDockStore.open
          ? 'Hide docked Live Stage monitor'
          : 'Show docked Live Stage monitor'}
        onclick={() => stageDockStore.toggle()}
      >
        STAGE
      </button>
    {/if}
    {#if !isWelcomeRoute}
      <button
        type="button"
        class="top-bar__rec"
        data-testid="top-bar-record"
        data-state={isRecording ? 'active' : 'off'}
        aria-pressed={isRecording ? 'true' : 'false'}
        title={isRecording
          ? 'Recording active — click to stop'
          : 'Recording status — click to start'}
        onclick={() => void handleRecordToggle()}
      >
        <span class="top-bar__rec-dot" aria-hidden="true"></span>
        <span>REC</span>
      </button>
      <button
        type="button"
        class="top-bar__dj"
        class:top-bar__dj--active={djAutopilotActive}
        data-testid="top-bar-dj-autopilot"
        aria-pressed={djAutopilotActive ? 'true' : 'false'}
        aria-label={djAutopilotActive ? 'DJ Autopilot active' : 'DJ Autopilot'}
        title={djAutopilotActive
          ? `DJ Autopilot active — auto-advances every ${djAutopilotStore.phraseIntervalBars} bars`
          : `Enable DJ Autopilot (auto-advance every ${djAutopilotStore.phraseIntervalBars} bars)`}
        onclick={() => djAutopilotStore.toggle()}
      >
        <span class="top-bar__dj-dot" aria-hidden="true"></span>
        <span>DJ</span>
      </button>
      <button
        type="button"
        class="top-bar__go-live"
        aria-pressed={outputWindowOpen ? 'true' : 'false'}
        data-live={outputWindowOpen ? 'on' : 'off'}
        title={outputWindowOpen
          ? goLiveArmed
            ? 'Click again to confirm closing live stage output'
            : 'Click twice to safely close live stage output'
          : goLiveArmed
            ? 'Click again to confirm opening live stage output'
            : 'Open stage output window on secondary display'}
        onclick={() => void handleGoLiveClick()}
      >
        <span class="top-bar__go-live-glyph"><Icon name="play" size={11} /></span>
        {outputWindowOpen
          ? goLiveArmed
            ? 'STOP LIVE?'
            : 'LIVE OUT'
          : goLiveArmed
            ? 'CONFIRM LIVE?'
            : 'GO LIVE'}
      </button>
    {/if}
  </div>
</header>

<style>
  .top-bar {
    position: relative;
    z-index: var(--z-topbar, 20);
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--space-md);
    padding: 0 var(--space-md);
    height: 52px;
    min-width: 0;
    background: var(--color-bg-1);
    border-bottom: 1px solid var(--color-line-1);
    color: var(--color-text-0);
    font-size: var(--font-size-body-sm);
  }

  .top-bar__brand {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  /* Brand mark — chromatic conic ring + dark inner disc + glowing accent
   * dot. Ported from `chromaticity-planning/ui-predev-bundle-v0.2-beta/
   * styles.css::.brand-mark`. The base is a 26px circle with a 5-color
   * conic gradient; ::after carves a dark inner disc; ::before places a
   * glowing primary-tinted dot at the center. Per-theme overrides below
   * tune the palette + glow per the bundle's pulse / atelier / console
   * theme rules. */
  .top-bar__brand-mark {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: conic-gradient(
      from 220deg at 50% 60%,
      var(--color-primary) 0deg,
      var(--color-signal-cyan) 90deg,
      var(--color-signal-magenta) 180deg,
      var(--color-signal-amber) 270deg,
      var(--color-primary) 360deg
    );
    position: relative;
    flex-shrink: 0;
  }

  .top-bar__brand-mark::after {
    content: '';
    position: absolute;
    inset: 5px;
    background: var(--color-bg-1);
    border-radius: 50%;
  }

  .top-bar__brand-mark::before {
    content: '';
    position: absolute;
    inset: 9px;
    background: var(--color-primary);
    border-radius: 50%;
    z-index: 1;
    box-shadow: 0 0 8px var(--color-primary);
  }

  /* Atelier (light/modernist): replace the chromatic ring with a solid
   * ink ring + solid bg-1 inner; drop the glow per the bundle. */
  :global([data-theme='atelier']) .top-bar__brand-mark {
    background: var(--color-text-0);
  }
  :global([data-theme='atelier']) .top-bar__brand-mark::before {
    background: var(--color-bg-1);
    box-shadow: none;
  }

  .top-bar__brand-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    line-height: 1;
  }

  /* Wordmark — chromatic gradient text under pulse via `background-clip:
   * text` per bundle's `[data-theme="pulse"] .tb-brand .name` rule.
   * Atelier renders a larger italic display face; console renders a
   * compact mono caps treatment (per the bundle's per-theme overrides).
   * The base style below is the pulse default; the two theme overrides
   * specialize. */
  .top-bar__wordmark {
    font-family: var(--font-family-display, var(--font-family-sans));
    font-size: 16px;
    font-weight: 700;
    letter-spacing: 0.01em;
    line-height: 1;
    white-space: nowrap;
    background: linear-gradient(
      90deg,
      var(--color-primary),
      var(--color-signal-cyan) 60%,
      var(--color-signal-magenta)
    );
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }

  :global([data-theme='atelier']) .top-bar__wordmark {
    font-family: var(--font-family-display, serif);
    font-size: 18px;
    font-weight: 500;
    font-style: italic;
    letter-spacing: -0.02em;
    background: none;
    -webkit-background-clip: unset;
    background-clip: unset;
    color: var(--color-text-0);
  }

  :global([data-theme='console']) .top-bar__wordmark {
    font-family: var(--font-family-mono);
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.04em;
    background: none;
    -webkit-background-clip: unset;
    background-clip: unset;
    color: var(--color-text-0);
    text-transform: uppercase;
  }

  .top-bar__title {
    color: var(--color-text-3);
    font-family: var(--font-family-mono);
    font-size: 9px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    line-height: 1;
    white-space: nowrap;
  }

  /* `.top-bar__center` reserves the middle column for the Wave 2.1
     Lane B.1 Perform variant toggle (STANDARD / CONSOLE) on the
     Perform route only. Left-aligned so the toggle sits adjacent to
     the brand mark rather than centered across the full 1fr column.
     Future B.2 transport HUD elements (timecode / beat dots / BPM /
     audio meters) compose alongside this toggle. */
  .top-bar__center {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    justify-self: start;
    min-width: 0;
    overflow: hidden;
  }

  .top-bar__variant {
    display: inline-flex;
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .top-bar__variant-btn {
    appearance: none;
    background: transparent;
    border: 0;
    padding: 4px var(--space-md);
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    cursor: pointer;
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }

  .top-bar__variant-btn:not(:last-child) {
    border-right: 1px solid var(--color-line-2);
  }

  .top-bar__variant-btn:hover {
    color: var(--color-text-0);
    background: var(--color-bg-3);
  }

  .top-bar__variant-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
    z-index: 1;
  }

  .top-bar__variant-btn--active {
    background: var(--color-primary);
    color: var(--color-on-primary, #fff);
  }

  .top-bar__variant-btn--active:hover {
    background: var(--color-primary);
    color: var(--color-on-primary, #fff);
    filter: brightness(1.08);
  }

  .top-bar__hud {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  /* W34a: the W15 PR1 inline `.top-bar__pill*` placeholder rules are
     dropped — pill rendering moves to the `<AdapterPill>` widget which
     owns its own styling. The container `.top-bar__pills` stays as a
     thin flex wrapper so the bundle's tb-section gap rhythm holds. */
  .top-bar__pills {
    display: flex;
    gap: var(--space-xs);
  }

  /* Wave 2.1 Lane B.2 — transport HUD cluster styles. The four
   * Perform-only chips (timecode / beat dots / BPM / VU meter) ride
   * inside `.top-bar__center` alongside the Lane B.1 variant toggle.
   * Each chip carries its own thin border for visual separation
   * against the dense central row, mirroring the bundle's `tb-stat` /
   * `tb-meta` chrome treatment in `Chromaticity.html` lines 203-222. */
  .top-bar__transport-timecode {
    display: inline-flex;
    align-items: center;
    padding: 2px var(--space-xs);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-md);
    font-variant-numeric: tabular-nums;
    color: var(--color-text-1);
    letter-spacing: 0.04em;
  }

  .top-bar__transport-timecode-text {
    font-family: var(--font-family-mono);
    font-variant-numeric: tabular-nums;
  }

  .top-bar__transport-beat {
    display: inline-flex;
    align-items: center;
  }

  .top-bar__bpm-anchor {
    position: relative;
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
  }

  .top-bar__transport-bpm {
    appearance: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px var(--space-xs);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    line-height: 1.2;
    white-space: nowrap;
    flex-shrink: 0;
    cursor: pointer;
    transition:
      border-color 120ms ease,
      background-color 120ms ease;
  }

  .top-bar__transport-bpm:hover {
    background: var(--color-bg-3);
    border-color: var(--color-line-3);
  }

  .top-bar__transport-bpm:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .top-bar__transport-bpm--locked {
    border-color: var(--color-signal-amber);
  }

  .top-bar__transport-bpm--off {
    border-color: var(--color-line-1);
    opacity: 0.78;
  }

  .top-bar__transport-bpm--open {
    background: var(--color-bg-3);
    border-color: var(--color-primary);
    opacity: 1;
  }

  .top-bar__transport-bpm-key {
    color: var(--color-text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .top-bar__transport-bpm-value {
    color: var(--color-primary);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .top-bar__transport-bpm-value--warming {
    color: var(--color-text-2);
  }

  .top-bar__transport-bpm-mode {
    padding: 1px 4px;
    border-radius: var(--radius-xs);
    background: var(--color-bg-3);
    color: var(--color-signal-cyan);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .top-bar__transport-bpm-mode--lock {
    background: var(--color-signal-amber);
    color: var(--color-bg-0);
  }

  .top-bar__transport-bpm-mode--off {
    background: var(--color-bg-3);
    color: var(--color-text-2);
  }

  .top-bar__beatmatch-popover {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: var(--z-overlay, 900);
    width: 312px;
    padding: var(--space-lg);
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow:
      0 14px 36px rgba(0, 0, 0, 0.55),
      var(--shadow-panel);
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    color: var(--color-text-0);
  }

  .beatmatch__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
    padding-bottom: var(--space-sm);
    border-bottom: 1px solid var(--color-line-1);
  }

  .beatmatch__title {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--color-text-1);
  }

  .beatmatch__status {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--color-signal-cyan);
  }

  .beatmatch__status--lock {
    color: var(--color-signal-amber);
  }

  .beatmatch__status--off {
    color: var(--color-text-2);
  }

  .beatmatch__section {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .beatmatch__label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-text-2);
  }

  .beatmatch__segmented {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .beatmatch__seg-btn {
    appearance: none;
    border: 0;
    background: transparent;
    padding: 6px var(--space-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    font-weight: 600;
    cursor: pointer;
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }

  .beatmatch__seg-btn:not(:last-child) {
    border-right: 1px solid var(--color-line-2);
  }

  .beatmatch__seg-btn:hover {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }

  .beatmatch__seg-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: -2px;
  }

  .beatmatch__seg-btn--active {
    background: var(--color-primary);
    color: var(--color-on-primary, #0c0e16);
  }

  .beatmatch__seg-btn--active:hover {
    background: var(--color-primary);
    color: var(--color-on-primary, #0c0e16);
    filter: brightness(1.06);
  }

  .beatmatch__tempo-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto auto;
    gap: var(--space-xs);
    align-items: center;
  }

  .beatmatch__bpm-input {
    width: 100%;
    min-width: 0;
    padding: 5px 8px;
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    color: var(--color-text-0);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-md);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .beatmatch__bpm-input:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .beatmatch__btn {
    appearance: none;
    padding: 5px 8px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    color: var(--color-text-0);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition:
      background-color 120ms ease,
      border-color 120ms ease;
  }

  .beatmatch__btn:hover {
    background: var(--color-bg-4);
    border-color: var(--color-line-3);
  }

  .beatmatch__btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .beatmatch__btn--accent {
    background: var(--color-bg-4);
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .beatmatch__btn--resync {
    background: var(--color-bg-4);
    border-color: var(--color-signal-cyan);
    color: var(--color-signal-cyan);
  }

  .beatmatch__phase-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-sm);
  }

  .beatmatch__phase-value {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--color-text-0);
  }

  .beatmatch__slider {
    width: 100%;
    margin: 2px 0;
    accent-color: var(--color-primary);
    cursor: pointer;
  }

  .beatmatch__slider:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .beatmatch__slider-ticks {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-family-mono);
    font-size: 9px;
    color: var(--color-text-2);
    font-variant-numeric: tabular-nums;
  }

  .beatmatch__nudge-row {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: var(--space-xs);
    margin-top: var(--space-xs);
  }

  .top-bar__transport-vu {
    display: inline-flex;
    align-items: center;
  }

  .top-bar__rec {
    appearance: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px var(--space-xs);
    background: transparent;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
    flex-shrink: 0;
    cursor: pointer;
    transition:
      border-color 120ms ease,
      background-color 120ms ease,
      color 120ms ease;
  }

  .top-bar__rec:hover {
    color: var(--color-text-0);
    background: var(--color-bg-3);
  }

  .top-bar__rec:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .top-bar__rec[data-state='active'] {
    border-color: var(--color-signal-red);
    color: var(--color-signal-red);
    background: color-mix(in srgb, var(--color-signal-red) 14%, transparent);
  }

  .top-bar__rec-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-3);
  }

  .top-bar__rec[data-state='active'] .top-bar__rec-dot {
    background: var(--color-signal-red);
    box-shadow: 0 0 6px var(--color-signal-red);
  }

  .top-bar__dj {
    appearance: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 7px;
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    font-weight: 700;
    color: var(--color-text-1);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    white-space: nowrap;
    flex-shrink: 0;
    cursor: pointer;
    transition:
      border-color 120ms ease,
      background-color 120ms ease,
      color 120ms ease;
  }

  .top-bar__dj:hover {
    color: var(--color-text-0);
    background: var(--color-bg-3);
    border-color: var(--color-line-3);
  }

  .top-bar__dj:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .top-bar__dj-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--color-text-3);
  }

  .top-bar__dj--active {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-on-primary, #0c0e16);
  }

  .top-bar__dj--active:hover {
    background: var(--color-primary);
    color: var(--color-on-primary, #0c0e16);
    filter: brightness(1.08);
  }

  .top-bar__dj--active .top-bar__dj-dot {
    background: currentColor;
  }

  .top-bar__metric {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-2);
  }

  .top-bar__metric-value {
    color: var(--color-text-0);
    font-weight: 600;
  }

  .top-bar__metric-unit {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-3);
  }

  .top-bar__stage-btn {
    padding: 3px 8px;
    background: var(--color-bg-2);
    color: var(--color-text-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    cursor: pointer;
    white-space: nowrap;
  }

  .top-bar__stage-btn:hover,
  .top-bar__stage-btn--active {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .top-bar__go-live-glyph {
    display: inline-flex;
    align-items: center;
    margin-right: 4px;
  }

  .top-bar__go-live {
    padding: 4px var(--space-md);
    background: color-mix(in srgb, var(--color-signal-red) 14%, var(--color-bg-2));
    color: var(--color-signal-red);
    border: 1px solid color-mix(in srgb, var(--color-signal-red) 50%, transparent);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    white-space: nowrap;
    flex-shrink: 0;
    cursor: pointer;
  }

  .top-bar__go-live[data-live='on'] {
    background: var(--color-signal-red);
    color: var(--color-on-primary, #fff);
    border-color: var(--color-signal-red);
  }

  .top-bar__go-live:hover {
    filter: brightness(1.1);
  }

  .top-bar__go-live:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  @media (max-width: 1380px) {
    .top-bar {
      gap: var(--space-sm);
      padding: 0 var(--space-sm);
    }
    .top-bar__title,
    .top-bar__transport-vu,
    .top-bar__rec {
      display: none;
    }
    .top-bar__center,
    .top-bar__hud {
      gap: 5px;
    }
    .top-bar__pills {
      gap: 4px;
    }
    .top-bar__variant-btn {
      padding: 4px var(--space-sm);
    }
    .top-bar__transport-timecode {
      padding: 2px 4px;
      font-size: var(--font-size-mono-sm);
    }
    .top-bar__transport-bpm {
      padding: 2px 5px;
      gap: 3px;
    }
    .top-bar__go-live {
      padding: 4px var(--space-sm);
    }
  }

  @media (max-width: 1120px) {
    .top-bar__transport-timecode,
    .top-bar__metric {
      display: none;
    }
    .top-bar__dj {
      padding: 3px 6px;
    }
  }
</style>
