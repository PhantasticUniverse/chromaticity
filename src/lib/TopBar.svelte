<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
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
  import { audioSnapshotStore } from './stores/audioSnapshot.svelte';
  import { loadedSceneStore } from './stores/loadedScene.svelte';
  import { midiStore } from './stores/midiStore.svelte';
  import { oscHealthStore } from './stores/oscHealth.svelte';
  import { performVariantStore, type PerformVariant } from './stores/performVariant.svelte';
  import { routeStore, type RouteName } from './stores/route.svelte';
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
  const currentBpm = $derived(audioBpmReady ? (audioSnapshot?.bpm ?? DEFAULT_BPM) : DEFAULT_BPM);
  // BPM display formatting: bundle renders one decimal place
  // (`Chromaticity.html` line 208 `128.0`). We mirror that.
  const bpmDisplay = $derived(audioSnapshot && !audioBpmReady ? '--' : currentBpm.toFixed(1));

  // Per-route subtitle. Hardcoded for W15; future enhancement could
  // accept a per-view dynamic count via route-store metadata (e.g.
  // Library passes "1,247 scenes / 12 authors" once a real corpus
  // populates).
  const ROUTE_TITLES: Readonly<Record<RouteName, string>> = {
    perform: 'Perform',
    library: 'Library',
    author: 'Author',
    'audio-uniforms': 'Audio Uniforms',
    playlist: 'Playlist',
    sources: 'Sources',
    midi: 'MIDI',
    osc: 'OSC · LISTENING — Port 7000',
    import: 'IMPORT',
    patch: 'PATCH',
    settings: 'Settings',
    license: 'License',
    welcome: 'Welcome',
    help: 'Help',
  };

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

  /** OUT pill state — `ok` whenever a scene is loaded into the runtime. */
  const outState: AdapterPillState = $derived(loadedSceneStore.snapshot !== null ? 'ok' : 'off');

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

  /** OUT pill tooltip detail — scene name when loaded; placeholder otherwise. */
  const outDetail = $derived(
    loadedSceneStore.snapshot ? `Scene: ${loadedSceneStore.snapshot.name}` : 'No scene loaded',
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
</script>

<header class="top-bar">
  <div class="top-bar__brand">
    <span class="top-bar__brand-mark" aria-hidden="true"></span>
    <span class="top-bar__brand-text">
      <span class="top-bar__wordmark">Chromaticity</span>
      <span class="top-bar__title">{ROUTE_TITLES[routeStore.current]}</span>
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
      <span class="top-bar__transport-bpm" data-testid="top-bar-bpm">
        <span class="top-bar__transport-bpm-key">BPM</span>
        <span
          class="top-bar__transport-bpm-value"
          class:top-bar__transport-bpm-value--warming={audioSnapshot !== null && !audioBpmReady}
        >
          {bpmDisplay}
        </span>
      </span>
      <span class="top-bar__transport-vu" data-testid="top-bar-vu" aria-label="Audio meter">
        <VuStereo width={88} />
      </span>
    {/if}
  </div>

  <div class="top-bar__hud">
    <div class="top-bar__pills" role="group" aria-label="Adapter status">
      <AdapterPill kind="OUT" state={outState} detail={outDetail} />
      <AdapterPill kind="AUDIO" state={audioState} detail={audioDetail} />
      <AdapterPill kind="MIDI" state={midiState} detail={midiDetail} />
      <AdapterPill kind="OSC" state={oscState} detail={oscDetail} />
    </div>
    <!-- Wave 2.3 Lane Q.2 (G.14) — bundle-faithful FPS placeholder.
         The bundle renders `61.0` (Chromaticity.html line 245) until
         real frame-rate telemetry wires via ADR-011 (Wave 2.2+). The
         W15 PR1 `—` placeholder made the chrome read as broken; the
         numeric stub mirrors the bundle so the topbar HUD reads
         structurally complete. -->
    <span class="top-bar__metric" title="Frames per second (bundle placeholder)">
      <span class="top-bar__metric-unit">fps</span>
      <span class="top-bar__metric-value">61.0</span>
    </span>
    <!-- Wave 2.1 Lane B.2 — GPU readout. Static placeholder matching the
         bundle's `Chromaticity.html` line 247 (`<span className="v
         accent">42%</span>`). Real GPU telemetry wires when ADR-011
         rendering backend lands (Wave 2.2+). -->
    <span class="top-bar__metric" title="GPU utilization (Wave 2.2+: real telemetry via ADR-011)">
      <span class="top-bar__metric-unit">gpu</span>
      <span class="top-bar__metric-value" data-testid="top-bar-gpu-value">42%</span>
    </span>
    {#if routeStore.current === 'perform'}
      <!-- Wave 2.1 Lane B.2 — camera + record buttons (visual stubs).
           Real wiring lands in Wave 2.2. Bundle adoption:
           `Chromaticity.html` lines 254-255. -->
      <button
        type="button"
        class="top-bar__icon-btn"
        data-testid="top-bar-camera"
        aria-label="Camera"
        title="Camera (Wave 2.2)"
      >
        <Icon name="camera" size={14} />
      </button>
      <button
        type="button"
        class="top-bar__icon-btn"
        data-testid="top-bar-record"
        aria-label="Record"
        title="Record (Wave 2.2)"
      >
        <Icon name="record" size={11} />
      </button>
    {/if}
    <span class="top-bar__rec" data-state="off" title="Recording (W15+ scope)">
      <span class="top-bar__rec-dot" aria-hidden="true"></span>
      <span>REC</span>
    </span>
    <button
      type="button"
      class="top-bar__go-live"
      title="Go live — visual stub in W15; output activation + entitlement check are post-MVP"
    >
      <span class="top-bar__go-live-glyph"><Icon name="play" size={11} /></span>
      GO LIVE
    </button>
  </div>
</header>

<style>
  .top-bar {
    display: grid;
    grid-template-columns: max-content 1fr max-content;
    align-items: center;
    gap: var(--space-md);
    padding: 0 var(--space-md);
    height: 52px;
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
    gap: var(--space-md);
    justify-self: start;
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

  .top-bar__transport-bpm {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    padding: 2px var(--space-xs);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
  }

  .top-bar__transport-bpm-key {
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .top-bar__transport-bpm-value {
    color: var(--color-primary);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .top-bar__transport-bpm-value--warming {
    color: var(--color-text-3);
  }

  .top-bar__transport-vu {
    display: inline-flex;
    align-items: center;
  }

  /* Wave 2.1 Lane B.2 — camera + record icon buttons in the right HUD
   * cluster. Visual stubs only; real wiring is Wave 2.2 scope. Match
   * the bundle's `btn ghost sm` / `btn danger sm` chrome compactly. */
  .top-bar__icon-btn {
    appearance: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--color-line-2);
    color: var(--color-text-1);
    border-radius: var(--radius-xs);
    padding: 4px 6px;
    cursor: pointer;
    transition:
      background-color 120ms ease,
      color 120ms ease;
  }

  .top-bar__icon-btn:hover {
    color: var(--color-text-0);
    background: var(--color-bg-3);
  }

  .top-bar__icon-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .top-bar__rec {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px var(--space-xs);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .top-bar__rec-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-3);
  }

  /* `data-state='active'` selectors land alongside the REC IPC wiring
     (W15+); kept off the stylesheet for now so svelte-check doesn't
     flag them as unused. The active rule will paint the pill border
     red and switch the dot to a signal-red glow. */

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

  .top-bar__go-live-glyph {
    display: inline-flex;
    align-items: center;
    margin-right: 4px;
  }

  .top-bar__go-live {
    padding: 4px var(--space-md);
    background: var(--color-signal-red);
    color: var(--color-on-primary, #fff);
    border: none;
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-sm);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
  }

  .top-bar__go-live:hover {
    filter: brightness(1.1);
  }

  .top-bar__go-live:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
</style>
