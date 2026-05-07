<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-18 PR3 MIDI view full port — promotes the previously-disabled
  // sidebar entry to a routed view per the canonical bundle JSX
  // (`chromaticity-planning/ui-predev-bundle-v0.2-beta/views/midi.jsx`,
  // the `MidiView` component mapped to `id: "midi"` in the bundle's
  // VIEWS array). Replaces the W16 PR2 sidebar stub. Wireframe-05
  // (`chromaticity-planning/ui-predev-bundle/screenshots/...`) is the
  // visual reference; the bundle JSX (which renders the same screenshot)
  // is the structural source per the wireframe-alignment memory
  // (`feedback_wireframes_normative_for_ui_ux.md`).
  //
  // Surfaces ported (mapping the bundle's M-01..M-14 callouts):
  //   M-01 Device allowlist — explicit per-device enable, no auto-claim.
  //   M-02 Tabs: Scene · Global · Meta · Media · Snaps · Start-scenes · Devices.
  //   M-03 Learn mode: target / last input / value + frozen waveform +
  //        history strip (last 5) + 4s idle-fallback hint.
  //   M-04 Relative encoder detection pills (auto / absolute / 7F-relative /
  //        2's-complement / binary-offset) with the bundle's "✓" affordance
  //        on the auto-detected option.
  //   M-05 14-bit pair flag pill on the binding row (MIDI-CONF-16 hover).
  //   M-06 NRPN/RPN indicator deferred — bundle does not surface a per-row
  //        flag for it in this view; M-06 is the reset-null-PN state chip
  //        which lives in the integration adapter (Phase-3+).
  //   M-07 MPE-config diagnostic banner (rendered above the binding panel
  //        when the Devices tab is active and any device's mpeWarned flag
  //        is true).
  //   M-08 MTC readout (right column).
  //   M-09 Snap page — 16 slots, slot 0 gold preset, mode disambiguation
  //        copy ("Apply" active by default).
  //   M-10 BPM Sync Source priority chain (Link / MIDI Clock / Audio / Tap).
  //   M-11 Saved · session-stable dot per binding row.
  //   M-12 Start-scenes tab (scene# → note bindings).
  //   M-13 Devices panel (allowlist toggles, vendor / port / map count,
  //        MPE pill on Seaboard).
  //   M-14 Virtual Pad mirror (W23 PR3 adopts the `CuePad` widget
  //        primitive — 4×8 grid of clickable pads with idle / armed /
  //        active / modulated visual states, click cycles state locally;
  //        no MIDI IPC dispatch — frozen-frame mirror).
  //
  // Locked W18 PR3 deviations from the bundle:
  //   - Bundle's `requestAnimationFrame` learn waveform → frozen inline
  //     SVG path sampled at `t ≈ 14..17 s` (mirrors the W17 PR3 Audio
  //     Uniforms spectrum precedent — no canvas, no rAF).
  //   - History strip's 900ms polling interval → static fixture (5 entries).
  //   - Learn mode 4s idle-fallback timer → static "warning" copy is not
  //     surfaced by default; the panel renders the canonical "waiting for
  //     input" state. The fallback prose is preserved as a tooltip on the
  //     learn-mode toggle for operator awareness.
  //   - BPM-source reorder ↑/↓ buttons → visual-only stubs (the priority
  //     chain is read-only frozen; real reorder lands with the integration
  //     adapter wiring).
  //   - Allowlist toggles, learn-mode begin/cancel, snap-mode picker, and
  //     the binding-row mode select all work locally as `$state`-backed
  //     UI; no IPC dispatch (post-Phase-3 EV-INTEGRATION-MATRIX scope).
  //   - Capability surfaces (`io.midi.input` / `io.midi.output` /
  //     `io.midi.learn`) annotate elements via `data-cap` attributes —
  //     rows are NOT hidden, per the brief's gating policy.
  //   - Bundle's `EmptyMidi` empty-state branch is omitted: the W18
  //     port is not wired to the brief's `useEmptyMode` toggle. The
  //     `EMPTY_MIDI_DEVICES` copy in `data/empty-states.ts` remains
  //     unchanged for future reuse if/when an empty-mode route lands.
  //   - W20 view-internal icon sweep: learn-mode toggle, learn-panel
  //     header / close, per-row remove, bang glyph, device-settings
  //     button, BPM-source reorder, snap-mode separators all render
  //     via the W19 PR3 `Icon.svelte` primitive (`x` / `link` / `bang`
  //     / `settings` / `caret` / `chevron`) per `views/midi.jsx`.
  import {
    BPM_SYNC_SOURCES,
    MIDI_CAPABILITY,
    MIDI_DEVICES,
    MIDI_HISTORY_FIXTURE,
    MIDI_LEARN_FIXTURE,
    MIDI_MTC,
    MIDI_ROWS_BY_TAB,
    MIDI_SNAP_SLOTS,
    MIDI_TABS,
    MIDI_TAB_HINTS,
    MPE_BANNER_COPY,
    RELATIVE_ENCODER_MODES,
    clampWaveformSample,
    midiLearnWaveformBase,
    type MidiDevice,
    type MidiBindingMode,
    type MidiBindingRow,
    type MidiHistoryEntry,
    type MidiTabId,
  } from '../data/midi-fixtures';
  import Icon from '../Icon.svelte';
  import { CuePad, type CuePadCell } from '../widgets';

  // Tab state, learn-mode state, snap-mode state, devices allowlist state.
  let activeTab = $state<MidiTabId>('global');
  let learning = $state<boolean>(false);
  let learnTarget = $state<string>(MIDI_LEARN_FIXTURE.target);
  let snapMode = $state<'apply' | 'save' | 'clear'>('apply');
  let routeFilter = $state<string>('');
  let devices = $state<MidiDevice[]>(MIDI_DEVICES.map((d) => ({ ...d })));

  // The bundle's 4s learn-idle timer is replaced by a frozen "no signal"
  // affordance on the toggle button title; toggling learn mode also
  // resets the displayed target to the binding the bundle picks per-tab
  // (`meta.hue` for Global, `${tab}.target` otherwise).
  function toggleLearn(): void {
    if (learning) {
      learning = false;
      return;
    }
    learning = true;
    learnTarget = activeTab === 'global' ? MIDI_LEARN_FIXTURE.target : `${activeTab}.target`;
  }

  function selectTab(id: MidiTabId): void {
    activeTab = id;
  }

  function toggleDevice(deviceId: string): void {
    devices = devices.map((d) => (d.id === deviceId ? { ...d, allowed: !d.allowed } : d));
  }

  // M-14 Virtual Pad mirror — W23 PR3 adopts the `CuePad` widget primitive;
  // W23 PR4 binds the pad's row/col dims to the selected pad-capable
  // controller's hardware grid (Launchpad X = 8×8 RGB grid, APC40 mkII =
  // 5×8 clip-launch grid) so the visual matches the device an operator
  // has plugged in, and refactors the seed to the bundle-faithful
  // `CuePadCell` object shape (per-pad hue) so playing pads render in
  // the Launchpad X rainbow palette. Click cycles a cell
  // `idle → armed → playing → idle`; no MIDI IPC dispatch (frozen frame,
  // awaits the integration adapter wiring).
  const padCapableDevices = $derived<readonly MidiDevice[]>(
    devices.filter((d) => d.padDims !== undefined),
  );
  let selectedPadDeviceId = $state<string>('dev.lpx');
  const selectedPadDevice = $derived<MidiDevice | undefined>(
    padCapableDevices.find((d) => d.id === selectedPadDeviceId) ?? padCapableDevices[0],
  );
  const padRows = $derived<number>(selectedPadDevice?.padDims?.rows ?? 8);
  const padCols = $derived<number>(selectedPadDevice?.padDims?.cols ?? 8);

  // Hue per pad index — palette spans 30 (orange) → 130 (green) matching
  // the Launchpad X RGB-LED rainbow rendered in the bundle's
  // `views/midi.jsx` Virtual Pad reference. Deterministic so the visual
  // is stable across reseeds (controller switches, etc).
  function hueFor(i: number): number {
    return 30 + ((i * 11) % 100);
  }
  function seedPadCells(rows: number, cols: number): CuePadCell[] {
    const total = rows * cols;
    return Array.from({ length: total }, (_, i): CuePadCell => {
      if (i % 5 === 0) return { state: 'playing', hue: hueFor(i) };
      if (i % 7 === 3) return { state: 'armed', hue: hueFor(i) };
      return {};
    });
  }
  let padState = $state<CuePadCell[]>(seedPadCells(8, 8));
  function selectPadDevice(deviceId: string): void {
    const next = padCapableDevices.find((d) => d.id === deviceId);
    if (!next?.padDims) return;
    selectedPadDeviceId = deviceId;
    padState = seedPadCells(next.padDims.rows, next.padDims.cols);
  }
  function cyclePad(row: number, col: number): void {
    const i = row * padCols + col;
    const next: CuePadCell[] = [...padState];
    const cell = next[i] ?? {};
    const currentState = cell.state ?? 'idle';
    const nextState: 'idle' | 'armed' | 'playing' =
      currentState === 'idle' ? 'armed' : currentState === 'armed' ? 'playing' : 'idle';
    next[i] = nextState === 'idle' ? {} : { state: nextState, hue: cell.hue ?? hueFor(i) };
    padState = next;
  }

  // Hidden in `snaps` and `devices` tabs — those render dedicated
  // panels.
  const showingBindingTable = $derived(activeTab !== 'snaps' && activeTab !== 'devices');
  const currentRows = $derived<readonly MidiBindingRow[]>(MIDI_ROWS_BY_TAB[activeTab]);
  const filteredRows = $derived.by<readonly MidiBindingRow[]>(() => {
    const q = routeFilter.trim().toLowerCase();
    if (!q) return currentRows;
    return currentRows.filter((row) =>
      [row.target, row.device, row.msg, row.num].join(' ').toLowerCase().includes(q),
    );
  });

  const mpeNotified = $derived(devices.some((d) => d.mpeWarned));

  // Live waveform + history ring buffers — W24 PR2 animates what the
  // W18 port shipped as a static snapshot. The bundle's
  // `MidiLearnWaveform` runs a ~50ms-step canvas ring buffer fed by
  // `0.45 + sin(t·0.9)·0.22 + sin(t·2.3)·0.08 + spike + noise`; we
  // mirror it as an SVG path driven by `$state` arrays so the
  // pre-ADR-011 no-canvas constraint stays intact while the visual
  // matches the bundle.
  //
  // `prefers-reduced-motion: reduce` short-circuits the animation to
  // the static fixture; the `$effect` only runs the rAF + setInterval
  // pair while the learn-mode panel is open (`learning === true`) so
  // closed-panel CPU is zero.
  const WAVE_W = 100;
  const WAVE_H = 32;
  const HISTORY_OPTIONS: readonly { readonly msg: string; readonly device: string }[] = [
    { msg: 'CC 15 MSB', device: 'APC40' },
    { msg: 'CC 47 LSB', device: 'APC40' },
    { msg: 'CC 14', device: 'APC40' },
    { msg: 'Note 42', device: 'LPX' },
    { msg: 'CC 21', device: 'LPX' },
  ];

  let liveWaveform = $state<readonly number[]>([...MIDI_LEARN_FIXTURE.waveform]);
  let liveHistory = $state<readonly MidiHistoryEntry[]>(
    MIDI_HISTORY_FIXTURE.map((e) => ({ ...e })),
  );
  let learnReducedMotion = $state<boolean>(false);

  $effect(() => {
    if (typeof window === 'undefined') return;
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    learnReducedMotion = mq.matches;
    const handler = (e: MediaQueryListEvent): void => {
      learnReducedMotion = e.matches;
    };
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  });

  $effect(() => {
    if (!learning) return;
    // Reset to fresh fixture state every time the panel re-opens so
    // operators see a clean baseline rather than the tail of an old
    // session.
    if (learnReducedMotion) {
      liveWaveform = [...MIDI_LEARN_FIXTURE.waveform];
      liveHistory = MIDI_HISTORY_FIXTURE.map((e) => ({ ...e }));
      return;
    }
    // Closure-local mirrors so the rAF / setInterval bodies don't read
    // `$state` (which would self-trigger the effect on each write).
    const samples = [...MIDI_LEARN_FIXTURE.waveform];
    let history = MIDI_HISTORY_FIXTURE.map((e) => ({ ...e }));
    liveWaveform = [...samples];
    liveHistory = [...history];

    let raf = 0;
    let lastSampleTime = 0;
    let entryCounter = 1000;
    const start = performance.now();

    const tick = (now: number): void => {
      if (now - lastSampleTime > 50) {
        lastSampleTime = now;
        // Sweep continues from the fixture's `t = 17 s` tail; using
        // `Math.random()` for spike + noise matches the bundle.
        const t = 17 + (now - start) / 1000;
        const spike = Math.random() < 0.08 ? Math.random() * 0.35 : 0;
        const noise = (Math.random() - 0.5) * 0.04;
        samples.shift();
        samples.push(clampWaveformSample(midiLearnWaveformBase(t) + spike + noise));
        liveWaveform = [...samples];
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    // Periodic event injection — bundle uses 900ms cadence (3000ms
    // under reduced motion, but we already short-circuit above).
    const injectInterval = window.setInterval(() => {
      const pick =
        HISTORY_OPTIONS[Math.floor(Math.random() * HISTORY_OPTIONS.length)] ?? HISTORY_OPTIONS[0]!;
      const val = Math.floor(Math.random() * 128);
      const next: MidiHistoryEntry = {
        id: `h-live-${entryCounter++}`,
        msg: pick.msg,
        device: pick.device,
        hex: '0x' + val.toString(16).padStart(2, '0'),
        value: val,
        relativeMs: 0,
      };
      history = [next, ...history.slice(0, 4)];
      liveHistory = [...history];
    }, 900);

    // "ago" label refresh — bundle ticks at 250ms so labels read
    // smoothly without per-frame DOM thrash.
    const tickInterval = window.setInterval(() => {
      history = history.map((e, i) => (i === 0 ? e : { ...e, relativeMs: e.relativeMs + 250 }));
      liveHistory = [...history];
    }, 250);

    return () => {
      cancelAnimationFrame(raf);
      window.clearInterval(injectInterval);
      window.clearInterval(tickInterval);
    };
  });

  const wavePath = $derived.by(() => {
    const samples = liveWaveform;
    if (samples.length === 0) return '';
    return samples
      .map((sample, i) => {
        const x = (i / (samples.length - 1)) * WAVE_W;
        const y = WAVE_H - sample * (WAVE_H - 2) - 1;
        return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  });

  // Filled-area polygon under the path (mirrors the bundle's translucent
  // accent-fill under the line).
  const waveFillPath = $derived.by(() => {
    const samples = liveWaveform;
    if (samples.length === 0) return '';
    const top = samples
      .map((sample, i) => {
        const x = (i / (samples.length - 1)) * WAVE_W;
        const y = WAVE_H - sample * (WAVE_H - 2) - 1;
        return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
    return `${top} L${WAVE_W},${WAVE_H} L0,${WAVE_H} Z`;
  });

  function lastWaveSampleY(): number {
    const samples = liveWaveform;
    if (samples.length === 0) return WAVE_H / 2;
    const last = samples[samples.length - 1] ?? 0;
    return WAVE_H - last * (WAVE_H - 2) - 1;
  }

  function bindingModeOptions(): readonly MidiBindingMode[] {
    return ['absolute', 'relative', 'toggle', 'momentary'] as const;
  }

  function relativeMs(ms: number): string {
    if (ms < 1000) return `+${ms}ms`;
    return `+${(ms / 1000).toFixed(1)}s`;
  }
</script>

<section class="midi" aria-label="MIDI mappings and devices">
  <div class="midi__main">
    <!-- Tabs (M-02) -->
    <header class="midi__tabs" role="tablist" aria-label="MIDI mapping tabs">
      {#each MIDI_TABS as tab (tab.id)}
        <button
          type="button"
          class="midi__tab"
          class:midi__tab--active={activeTab === tab.id}
          role="tab"
          aria-selected={activeTab === tab.id ? 'true' : 'false'}
          tabindex={activeTab === tab.id ? 0 : -1}
          onclick={() => selectTab(tab.id)}
        >
          {tab.label}
        </button>
      {/each}
    </header>

    <!-- M-07 MPE-config banner (Devices tab only, gated on mpeNotified) -->
    {#if mpeNotified && activeTab === 'devices'}
      <div class="midi__banner midi__banner--info" role="status">
        <span class="midi__banner-text">{MPE_BANNER_COPY}</span>
      </div>
    {/if}

    <!-- Learn-mode toggle row + tab-specific hint + filter input -->
    <div class="midi__toolbar">
      <button
        type="button"
        class="midi__btn"
        class:midi__btn--primary={!learning}
        class:midi__btn--danger={learning}
        aria-pressed={learning ? 'true' : 'false'}
        data-cap={MIDI_CAPABILITY.learn}
        onclick={toggleLearn}
        title={learning
          ? 'Cancel learn mode'
          : 'Click to map — 4s idle fallback offers manual binding when no signal arrives'}
      >
        {#if learning}
          <Icon name="x" size={12} /> Cancel Learn
        {:else}
          <Icon name="link" size={12} /> Click to map
        {/if}
      </button>

      {#if learning}
        <span class="midi__toolbar-hint midi__toolbar-hint--learn">
          <Icon name="link" size={11} /> Waiting for input — wiggle a CC or press a note.
        </span>
      {:else}
        <span class="midi__toolbar-hint">{MIDI_TAB_HINTS[activeTab]}</span>
      {/if}

      <span class="midi__toolbar-spacer" aria-hidden="true"></span>

      <label class="midi__filter">
        <span class="midi__caps">filter</span>
        <input
          type="search"
          class="midi__input"
          placeholder="target or device…"
          aria-label="filter MIDI bindings"
          bind:value={routeFilter}
        />
      </label>
    </div>

    <!-- Body: dedicated panels for snaps + devices, table + live controls otherwise -->
    <div class="midi__body">
      {#if activeTab === 'devices'}
        <div class="midi__devices-wrap">
          <article class="midi__panel">
            <header class="midi__panel-head">
              <h2 class="midi__panel-title">Devices · allowlist</h2>
              <span class="midi__panel-sub">M-01 · explicit enable only, no auto-claim</span>
            </header>
            <div class="midi__devices">
              <div class="midi__devices-head">
                <span aria-hidden="true"></span>
                <span class="midi__caps">name</span>
                <span class="midi__caps">vendor</span>
                <span class="midi__caps">ports</span>
                <span class="midi__caps">maps</span>
                <span class="midi__caps">allow</span>
                <span aria-hidden="true"></span>
              </div>
              <ul class="midi__devices-list">
                {#each devices as d (d.id)}
                  <li
                    class="midi__device-row"
                    class:midi__device-row--off={!d.allowed}
                    data-cap={MIDI_CAPABILITY.input}
                  >
                    <span
                      class="midi__device-dot"
                      class:midi__device-dot--on={d.allowed}
                      aria-hidden="true"
                    ></span>
                    <span class="midi__device-name">
                      <span class="mono">{d.name}</span>
                      {#if d.mpeWarned}
                        <span
                          class="midi__pill midi__pill--info"
                          title="MPE-configuration message seen — no global state corrupted (MIDI-CONF-18)"
                        >
                          MPE
                        </span>
                      {/if}
                    </span>
                    <span class="midi__device-vendor mono">{d.vendor}</span>
                    <span class="midi__device-dir mono">{d.direction}</span>
                    <span
                      class="midi__device-maps mono"
                      class:midi__device-maps--zero={d.maps === 0}
                    >
                      {d.maps}
                    </span>
                    <span class="midi__device-toggle">
                      <button
                        type="button"
                        class="midi__device-toggle-btn"
                        class:midi__device-toggle-btn--on={d.allowed}
                        aria-pressed={d.allowed ? 'true' : 'false'}
                        aria-label="{d.allowed ? 'Disallow' : 'Allow'} {d.name}"
                        onclick={() => toggleDevice(d.id)}
                      >
                        <span class="midi__device-toggle-knob" aria-hidden="true"></span>
                      </button>
                    </span>
                    <button
                      type="button"
                      class="midi__device-settings"
                      aria-label="Device settings — {d.name}"
                      title="Device details"
                    >
                      <Icon name="settings" size={11} />
                    </button>
                  </li>
                {/each}
              </ul>
            </div>
          </article>
        </div>
      {:else if activeTab === 'snaps'}
        <!-- M-09 Snap page -->
        <div class="midi__snaps-wrap">
          <header class="midi__snaps-head">
            <h2 class="midi__snaps-title">Snap slots</h2>
            <span class="midi__snaps-sub">
              16 slots per show · slot 0 is gold preset (CP-05) · wire literal:
              <code class="midi__snaps-wire">favslot</code>
            </span>
            <span class="midi__toolbar-spacer" aria-hidden="true"></span>
            <div class="midi__snap-mode">
              <span class="midi__caps">mode</span>
              <button
                type="button"
                class="midi__snap-mode-btn"
                class:midi__snap-mode-btn--active={snapMode === 'apply'}
                aria-pressed={snapMode === 'apply' ? 'true' : 'false'}
                onclick={() => (snapMode = 'apply')}>Apply</button
              >
              <span class="midi__snap-mode-sep" aria-hidden="true">·</span>
              <button
                type="button"
                class="midi__snap-mode-btn"
                class:midi__snap-mode-btn--active={snapMode === 'save'}
                aria-pressed={snapMode === 'save' ? 'true' : 'false'}
                onclick={() => (snapMode = 'save')}>Save</button
              >
              <span class="midi__snap-mode-sep" aria-hidden="true">·</span>
              <button
                type="button"
                class="midi__snap-mode-btn"
                class:midi__snap-mode-btn--active={snapMode === 'clear'}
                aria-pressed={snapMode === 'clear' ? 'true' : 'false'}
                onclick={() => (snapMode = 'clear')}>Clear</button
              >
            </div>
          </header>

          <ul class="midi__snap-grid">
            {#each MIDI_SNAP_SLOTS as slot (slot.n)}
              <li>
                <button
                  type="button"
                  class="midi__snap"
                  class:midi__snap--gold={slot.isGold}
                  class:midi__snap--active={slot.isActive}
                  class:midi__snap--bound={slot.binding !== null}
                  aria-pressed={slot.isActive ? 'true' : 'false'}
                  title={slot.binding
                    ? `Wire: /favslots/${slot.n}  ·  ${slot.binding}`
                    : `Unbound · /favslots/${slot.n}`}
                >
                  <span class="midi__snap-row">
                    <span class="midi__snap-num mono">
                      {String(slot.n).padStart(2, '0')}
                    </span>
                    {#if slot.binding}
                      <span class="midi__snap-mode-pill mono">{slot.mode}</span>
                    {/if}
                  </span>
                  <span class="midi__snap-name" class:midi__snap-name--empty={!slot.name}>
                    {slot.name || 'empty'}
                  </span>
                  <span
                    class="midi__snap-binding mono"
                    class:midi__snap-binding--unbound={!slot.binding}
                  >
                    {slot.binding ?? 'unbound'}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {:else if showingBindingTable}
        <!-- Scene / Global / Meta / Media / Start tabs render the binding table -->
        <div class="midi__bindings">
          <table class="midi__table">
            <colgroup>
              <col class="midi__col-target" />
              <col class="midi__col-device" />
              <col class="midi__col-ch" />
              <col class="midi__col-msg" />
              <col class="midi__col-num" />
              <col class="midi__col-mode" />
              <col class="midi__col-min" />
              <col class="midi__col-max" />
              <col class="midi__col-thresh" />
              <col class="midi__col-flag" />
              <col class="midi__col-actions" />
            </colgroup>
            <thead>
              <tr>
                <th scope="col">target</th>
                <th scope="col">device</th>
                <th scope="col">ch</th>
                <th scope="col">msg</th>
                <th scope="col">num</th>
                <th scope="col">mode</th>
                <th scope="col">min</th>
                <th scope="col">max</th>
                <th scope="col">thresh</th>
                <th scope="col">flag</th>
                <th scope="col" aria-label="actions"></th>
              </tr>
            </thead>
            <!--
              TODO(scale): adopt @tanstack/svelte-virtual (or equivalent) when binding
              cardinality exceeds ~200 rows. The MIDI binding table currently renders
              full DOM. Pre-flag for the operator-cohort residency.
            -->
            <tbody>
              {#each filteredRows as row (`${row.target}-${row.device}-${row.num}`)}
                <tr
                  class:midi__row--learning={learning && row.target === learnTarget}
                  data-family={row.family}
                  data-cap={MIDI_CAPABILITY.input}
                >
                  <th scope="row" class="midi__cell midi__cell--target mono">{row.target}</th>
                  <td class="midi__cell mono">{row.device}</td>
                  <td class="midi__cell mono">{row.channel}</td>
                  <td class="midi__cell mono">{row.msg}</td>
                  <td class="midi__cell mono">{row.num}</td>
                  <td class="midi__cell midi__cell--mode">
                    <select class="midi__select mono" aria-label="binding mode for {row.target}">
                      {#each bindingModeOptions() as opt (opt)}
                        <option value={opt} selected={opt === row.mode}>{opt}</option>
                      {/each}
                    </select>
                  </td>
                  <td class="midi__cell mono">{row.min}</td>
                  <td class="midi__cell mono">{row.max}</td>
                  <td class="midi__cell mono">{row.threshold}</td>
                  <td class="midi__cell midi__cell--flag">
                    {#if row.flag === '14-bit'}
                      <span
                        class="midi__pill midi__pill--info"
                        title="MSB/LSB 14-bit pair · implicit LSB=0 at 50ms (MIDI-CONF-16)"
                      >
                        14-bit
                      </span>
                    {/if}
                    {#if row.flag === "2's-comp"}
                      <span
                        class="midi__pill midi__pill--success"
                        title="Relative encoder auto-detected: 2's-complement (M-04)"
                      >
                        rel 2's
                      </span>
                    {/if}
                    <span
                      class="midi__saved-dot"
                      role="img"
                      title="Saved · session-stable (MIDI-CONF-20)"
                      aria-label="saved · session-stable"
                    ></span>
                  </td>
                  <td class="midi__cell midi__cell--actions">
                    <button
                      type="button"
                      class="midi__btn midi__btn--icon midi__btn--danger"
                      aria-label="Remove binding {row.target}"
                      title="Remove binding"
                    >
                      <Icon name="x" size={10} />
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>

          {#if filteredRows.length === 0}
            <p class="midi__no-rows mono">no bindings match filter</p>
          {/if}

          <!-- Live Scene Controls block: rendered for scene + global tabs only,
               mirroring the bundle's `(tab === "scene" || tab === "global")` guard. -->
          {#if activeTab === 'scene' || activeTab === 'global'}
            <section class="midi__live" aria-label="Live scene controls">
              <h3 class="midi__subhead">Live Scene Controls</h3>
              <div class="midi__live-grid">
                <article class="midi__group" style="--group-tint: var(--color-cat-color);">
                  <header class="midi__group-head">toggle</header>
                  <div class="midi__group-body">
                    {#each [0, 1, 2, 3] as i (i)}
                      <div class="midi__widget midi__widget--toggle">
                        <button
                          type="button"
                          class="midi__toggle"
                          class:midi__toggle--on={i % 2 === 0}
                          aria-pressed={i % 2 === 0 ? 'true' : 'false'}
                          aria-label="TOGGLE {i}"
                        >
                          <span class="midi__toggle-knob" aria-hidden="true"></span>
                        </button>
                        <span class="midi__widget-label mono">TOGGLE {i}</span>
                      </div>
                    {/each}
                  </div>
                </article>

                <article class="midi__group" style="--group-tint: var(--color-cat-motion);">
                  <header class="midi__group-head">slider</header>
                  <div class="midi__group-body">
                    {#each [0.13, 0.25, 0.37, 0.5] as v, i (i)}
                      <div class="midi__widget midi__widget--slider">
                        <div class="midi__fader" aria-hidden="true">
                          <div
                            class="midi__fader-fill"
                            style="height: {Math.round(v * 100)}%;"
                          ></div>
                        </div>
                        <span class="midi__widget-label mono">SLIDER {i}</span>
                      </div>
                    {/each}
                  </div>
                </article>

                <article class="midi__group" style="--group-tint: var(--color-cat-fx);">
                  <header class="midi__group-head">bang</header>
                  <div class="midi__group-body">
                    {#each [0, 1, 2, 3] as i (i)}
                      <div class="midi__widget midi__widget--bang">
                        <button type="button" class="midi__bang" aria-label="BANG {i}">
                          <span class="midi__bang-glyph"><Icon name="bang" size={22} /></span>
                        </button>
                        <span class="midi__widget-label mono">BANG {i}</span>
                      </div>
                    {/each}
                  </div>
                </article>

                <article class="midi__group" style="--group-tint: var(--color-cat-media);">
                  <header class="midi__group-head">knob</header>
                  <div class="midi__group-body">
                    {#each [0.13, 0.25, 0.38, 0.5] as v, i (i)}
                      <div class="midi__widget midi__widget--knob">
                        <div
                          class="midi__knob"
                          aria-hidden="true"
                          style="--knob-rot: {-135 + v * 270}deg;"
                        >
                          <span class="midi__knob-indicator"></span>
                        </div>
                        <span class="midi__widget-label mono">KNOB {i}</span>
                      </div>
                    {/each}
                  </div>
                </article>
              </div>
            </section>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- Right column: learn-mode panel (gated on `learning`), BPM sync, MTC,
       Virtual Pad mirror. -->
  <aside class="midi__side" aria-label="MIDI right column">
    {#if learning}
      <article class="midi__panel midi__panel--learn">
        <header class="midi__panel-head midi__panel-head--learn">
          <h2 class="midi__panel-title midi__panel-title--learn">
            <Icon name="link" size={14} /> Learn Mode
          </h2>
          <button
            type="button"
            class="midi__btn midi__btn--icon"
            aria-label="Close learn panel"
            onclick={() => (learning = false)}
          >
            <Icon name="x" size={11} />
          </button>
        </header>
        <div class="midi__panel-body">
          <div class="midi__learn-row">
            <span class="midi__caps">target</span>
            <span class="midi__learn-target mono">{learnTarget}</span>
          </div>

          <div class="midi__learn-grid">
            <div class="midi__learn-cell">
              <span class="midi__caps">last input</span>
              <span class="midi__learn-text mono">
                {MIDI_LEARN_FIXTURE.lastInputDevice} · CH {MIDI_LEARN_FIXTURE.lastInputChannel}
                · {MIDI_LEARN_FIXTURE.lastInputMessage}
              </span>
            </div>
            <div class="midi__learn-cell">
              <span class="midi__caps">value</span>
              <span class="midi__learn-text mono">
                {MIDI_LEARN_FIXTURE.valueHex} ({MIDI_LEARN_FIXTURE.valueDec})
              </span>
            </div>
          </div>

          <div class="midi__learn-cell">
            <span class="midi__caps">incoming signal</span>
            <div
              class="midi__waveform"
              role="img"
              aria-label="MIDI activity waveform (frozen frame)"
            >
              <svg
                class="midi__waveform-svg"
                viewBox="0 0 {WAVE_W} {WAVE_H}"
                preserveAspectRatio="none"
                aria-hidden="true"
              >
                {#each [1, 2, 3] as line (line)}
                  <line
                    class="midi__waveform-grid"
                    x1="0"
                    x2={WAVE_W}
                    y1={(WAVE_H * line) / 4}
                    y2={(WAVE_H * line) / 4}
                  />
                {/each}
                <path class="midi__waveform-fill" d={waveFillPath} />
                <path class="midi__waveform-line" d={wavePath} vector-effect="non-scaling-stroke" />
                <circle class="midi__waveform-head" cx={WAVE_W} cy={lastWaveSampleY()} r="1.4" />
              </svg>
            </div>
          </div>

          <!-- M-04 relative encoder detection pills -->
          <div class="midi__learn-cell">
            <span class="midi__caps">relative encoder detection</span>
            <ul class="midi__encoder-pills">
              {#each RELATIVE_ENCODER_MODES as mode (mode)}
                <li>
                  <button
                    type="button"
                    class="midi__pill midi__pill--encoder"
                    class:midi__pill--encoder-active={mode === MIDI_LEARN_FIXTURE.detectedMode}
                    aria-pressed={mode === MIDI_LEARN_FIXTURE.detectedMode ? 'true' : 'false'}
                  >
                    {mode}{mode === MIDI_LEARN_FIXTURE.detectedMode ? ' ✓' : ''}
                  </button>
                </li>
              {/each}
            </ul>
          </div>

          <!-- M-03 history strip (last 5) -->
          <div class="midi__learn-cell">
            <span class="midi__caps">history · last 5</span>
            <ul class="midi__history">
              {#each liveHistory as entry, i (entry.id)}
                <li class="midi__history-row" class:midi__history-row--head={i === 0}>
                  <span class="midi__history-msg mono">{entry.msg}</span>
                  <span class="midi__history-dev mono">{entry.device}</span>
                  <span class="midi__history-hex mono">{entry.hex}</span>
                  <span class="midi__history-ago mono">{relativeMs(entry.relativeMs)}</span>
                </li>
              {/each}
            </ul>
          </div>

          <div class="midi__learn-actions">
            <button type="button" class="midi__btn midi__btn--primary midi__btn--grow">
              Bind
            </button>
            <button type="button" class="midi__btn" onclick={() => (learning = false)}>
              Skip
            </button>
          </div>
        </div>
      </article>
    {/if}

    <!-- M-10 BPM Sync Source -->
    <article class="midi__panel">
      <header class="midi__panel-head">
        <h2 class="midi__panel-title">BPM Sync Source</h2>
        <span class="midi__panel-active mono">
          ● active: {BPM_SYNC_SOURCES.find((s) => s.state === 'active')?.name ?? '—'}
        </span>
      </header>
      <div class="midi__panel-body">
        <p class="midi__panel-prose mono">
          Priority chain — first active source wins. Drag to reorder.
        </p>
        <ul class="midi__bpm-list">
          {#each BPM_SYNC_SOURCES as src, i (src.id)}
            <li
              class="midi__bpm-row"
              class:midi__bpm-row--active={src.state === 'active'}
              data-state={src.state}
            >
              <span class="midi__bpm-dot" aria-hidden="true"></span>
              <span class="midi__bpm-rank mono">{i + 1}</span>
              <span class="midi__bpm-name">
                {src.name}
                <span class="midi__bpm-detail mono">{src.detail}</span>
              </span>
              <span class="midi__bpm-actions">
                <button
                  type="button"
                  class="midi__btn midi__btn--icon midi__btn--icon-up"
                  aria-label="Move {src.name} up"
                  disabled={i === 0}
                  title="Reorder — wired post-MVP"
                >
                  <Icon name="caret" size={11} />
                </button>
                <button
                  type="button"
                  class="midi__btn midi__btn--icon midi__btn--icon-down"
                  aria-label="Move {src.name} down"
                  disabled={i === BPM_SYNC_SOURCES.length - 1}
                  title="Reorder — wired post-MVP"
                >
                  <Icon name="caret" size={11} />
                </button>
              </span>
            </li>
          {/each}
        </ul>
      </div>
    </article>

    <!-- M-08 MTC readout -->
    <article class="midi__panel">
      <header class="midi__panel-head">
        <h2 class="midi__panel-title">MIDI Time Code</h2>
        <span class="midi__panel-sub">MIDI-CONF-19</span>
      </header>
      <div class="midi__panel-body">
        <div class="midi__mtc">
          <span class="midi__mtc-tc mono">{MIDI_MTC.timecode}</span>
          <span class="midi__mtc-fps mono">{MIDI_MTC.framerate}</span>
        </div>
        <p class="midi__mtc-prose mono">
          quarter-frame assembly: {MIDI_MTC.assemblyState} · exposed to scenes as
          <span class="midi__mtc-uniform">{MIDI_MTC.uniformName}</span>
        </p>
      </div>
    </article>

    <!-- M-14 Virtual Pad -->
    <article class="midi__panel">
      <header class="midi__panel-head">
        <h2 class="midi__panel-title">Virtual Pad</h2>
        <label class="midi__pad-controller">
          <span class="visually-hidden">Mirrored controller</span>
          <select
            class="midi__pad-controller-select"
            value={selectedPadDeviceId}
            onchange={(e) => selectPadDevice(e.currentTarget.value)}
            aria-label="Mirrored pad controller"
          >
            {#each padCapableDevices as device (device.id)}
              <option value={device.id}>{device.name}</option>
            {/each}
          </select>
          <span class="midi__pad-controller-mode">· mirror</span>
        </label>
      </header>
      <div class="midi__panel-body" data-cap={MIDI_CAPABILITY.output}>
        <CuePad rows={padRows} cols={padCols} cells={padState} onPadClick={cyclePad} />
      </div>
    </article>
  </aside>
</section>

<style>
  /* ---------------- Layout ----------------------------------------- */
  .midi {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 420px;
    gap: var(--space-md);
    align-items: start;
    min-height: 0;
  }

  @media (max-width: 1100px) {
    .midi {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* ---------------- Main panel ------------------------------------- */
  .midi__main {
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .midi__tabs {
    display: flex;
    gap: var(--space-xs);
    flex-wrap: wrap;
    padding: var(--space-xs);
    background: var(--color-bg-3);
    border-bottom: 1px solid var(--color-line-2);
  }

  .midi__tab {
    padding: var(--space-xs) var(--space-md);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
    transition:
      background 120ms ease,
      color 120ms ease,
      border-color 120ms ease;
  }
  .midi__tab:hover {
    color: var(--color-text-0);
    background: var(--color-bg-2);
  }
  .midi__tab:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .midi__tab--active {
    background: var(--color-bg-1);
    color: var(--color-primary);
    border-color: var(--color-line-3);
  }

  .midi__banner {
    padding: var(--space-sm) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  .midi__banner--info {
    background: color-mix(in oklab, var(--color-info) 10%, var(--color-bg-2));
    color: var(--color-info);
  }
  .midi__banner-text {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
  }

  .midi__toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
    padding: var(--space-md) var(--space-lg);
    border-bottom: 1px solid var(--color-line-2);
    background: var(--color-bg-1);
  }
  .midi__toolbar-spacer {
    flex: 1;
  }
  .midi__toolbar-hint {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
  }
  .midi__toolbar-hint--learn {
    color: var(--color-primary);
  }

  .midi__filter {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
  }
  .midi__input {
    padding: 2px var(--space-xs);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    color: var(--color-text-0);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    width: 200px;
  }
  .midi__input:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .midi__caps {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
  }

  /* ---------------- Buttons ---------------------------------------- */
  .midi__btn {
    padding: var(--space-xs) var(--space-md);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }
  .midi__btn:hover:not(:disabled) {
    background: var(--color-bg-4);
    color: var(--color-text-0);
  }
  .midi__btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .midi__btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .midi__btn--primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }
  .midi__btn--primary:hover:not(:disabled) {
    filter: brightness(1.05);
    color: var(--color-on-primary);
    background: var(--color-primary);
  }
  .midi__btn--danger {
    background: var(--color-danger);
    color: var(--color-on-primary);
    border-color: var(--color-danger);
  }
  .midi__btn--danger:hover:not(:disabled) {
    filter: brightness(1.05);
    color: var(--color-on-primary);
    background: var(--color-danger);
  }
  .midi__btn--icon {
    padding: 2px var(--space-xs);
    min-width: 22px;
    line-height: 1;
  }
  /* Reuse the chevron-down `caret` glyph for the BPM reorder pair —
     flipping the up button via transform keeps the icon vocabulary
     bundle-aligned without introducing a new icon name. */
  .midi__btn--icon-up :global(svg) {
    transform: rotate(180deg);
  }
  .midi__btn--grow {
    flex: 1;
  }

  /* ---------------- Body ------------------------------------------- */
  .midi__body {
    overflow: auto;
    min-height: 0;
    flex: 1;
  }

  /* ---------------- Devices panel ---------------------------------- */
  .midi__devices-wrap {
    padding: var(--space-md) var(--space-lg);
  }

  .midi__panel {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .midi__panel-head {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg-3);
    border-bottom: 1px solid var(--color-line-2);
  }
  .midi__panel-title {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-1);
  }
  .midi__panel-sub {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
    margin-left: auto;
  }
  .midi__pad-controller {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: var(--space-xxs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
  }
  .midi__pad-controller-select {
    appearance: none;
    background: transparent;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: 2px var(--space-sm) 2px var(--space-xs);
    color: var(--color-text-2);
    font-family: inherit;
    font-size: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    cursor: pointer;
    background-image:
      linear-gradient(45deg, transparent 50%, var(--color-text-3) 50%),
      linear-gradient(135deg, var(--color-text-3) 50%, transparent 50%);
    background-position:
      calc(100% - 8px) 50%,
      calc(100% - 4px) 50%;
    background-size:
      4px 4px,
      4px 4px;
    background-repeat: no-repeat;
    padding-right: 16px;
  }
  .midi__pad-controller-select:hover,
  .midi__pad-controller-select:focus-visible {
    border-color: var(--color-line-1);
    color: var(--color-text-1);
    outline: none;
  }
  .midi__pad-controller-select:focus-visible {
    border-color: var(--color-primary);
  }
  .midi__pad-controller-mode {
    color: var(--color-text-3);
  }
  .visually-hidden {
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
  .midi__panel-active {
    color: var(--color-primary);
    margin-left: auto;
    font-size: var(--font-size-mono-sm);
  }
  .midi__panel-body {
    padding: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }
  .midi__panel-prose {
    margin: 0;
    color: var(--color-text-3);
    font-size: var(--font-size-mono-sm);
  }

  .midi__devices {
    display: grid;
    grid-template-columns: 14px 1.6fr 1fr 64px 60px 70px 24px;
    gap: var(--space-sm);
    align-items: center;
    padding: var(--space-sm) var(--space-md);
  }
  .midi__devices-head {
    display: contents;
  }
  .midi__devices-list {
    display: contents;
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .midi__device-row {
    display: contents;
  }
  .midi__device-row > * {
    border-top: 1px solid var(--color-line-1);
    padding: var(--space-xs) 0;
  }
  .midi__device-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--color-text-4);
    align-self: center;
    border-top: none;
    margin-top: 4px;
  }
  .midi__device-dot--on {
    background: var(--color-success);
    box-shadow: 0 0 calc(4px * var(--glow, 1)) var(--color-success);
  }
  .midi__device-name {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
  }
  .midi__device-row--off .midi__device-name {
    color: var(--color-text-3);
  }
  .midi__device-vendor,
  .midi__device-dir,
  .midi__device-maps {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-2);
  }
  .midi__device-maps {
    color: var(--color-primary);
    text-align: right;
  }
  .midi__device-maps--zero {
    color: var(--color-text-4);
  }
  .midi__device-toggle {
    display: flex;
    align-items: center;
  }
  .midi__device-toggle-btn {
    width: 32px;
    height: 16px;
    border-radius: var(--radius-full);
    background: var(--color-bg-4);
    border: 1px solid var(--color-line-2);
    position: relative;
    cursor: pointer;
    padding: 0;
    transition: background 120ms ease;
  }
  .midi__device-toggle-btn--on {
    background: var(--color-success);
    border-color: var(--color-success);
  }
  .midi__device-toggle-knob {
    position: absolute;
    top: 1px;
    left: 1px;
    width: 12px;
    height: 12px;
    background: var(--color-text-0);
    border-radius: 50%;
    transition: transform 120ms ease;
  }
  .midi__device-toggle-btn--on .midi__device-toggle-knob {
    transform: translateX(16px);
  }
  .midi__device-toggle-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .midi__device-settings {
    background: transparent;
    border: 0;
    color: var(--color-text-3);
    cursor: pointer;
    font-size: var(--font-size-caps-md);
  }
  .midi__device-settings:hover {
    color: var(--color-text-0);
  }

  /* ---------------- Pills ------------------------------------------ */
  .midi__pill {
    display: inline-flex;
    align-items: center;
    padding: 1px 6px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
  }
  .midi__pill--info {
    background: color-mix(in oklab, var(--color-info) 14%, transparent);
    border-color: var(--color-info);
    color: var(--color-info);
  }
  .midi__pill--success {
    background: color-mix(in oklab, var(--color-success) 14%, transparent);
    border-color: var(--color-success);
    color: var(--color-success);
  }
  .midi__pill--encoder {
    cursor: pointer;
  }
  .midi__pill--encoder-active {
    background: color-mix(in oklab, var(--color-success) 14%, transparent);
    border-color: var(--color-success);
    color: var(--color-success);
  }

  /* ---------------- Snap grid (M-09) ------------------------------- */
  .midi__snaps-wrap {
    padding: var(--space-md) var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .midi__snaps-head {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }
  .midi__snaps-title {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-1);
  }
  .midi__snaps-sub {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
  }
  .midi__snaps-wire {
    background: var(--color-bg-3);
    padding: 1px 4px;
    border-radius: var(--radius-xs);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }

  .midi__snap-mode {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
  }
  .midi__snap-mode-btn {
    padding: 2px 6px;
    background: transparent;
    border: 0;
    color: var(--color-text-3);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    cursor: pointer;
  }
  .midi__snap-mode-btn--active {
    color: var(--color-primary);
  }
  .midi__snap-mode-btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .midi__snap-mode-sep {
    color: var(--color-text-4);
  }

  .midi__snap-grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-sm);
  }
  @media (max-width: 800px) {
    .midi__snap-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .midi__snap {
    width: 100%;
    text-align: left;
    background: var(--color-bg-2);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-sm) var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    min-height: 64px;
    cursor: pointer;
    color: var(--color-text-1);
  }
  .midi__snap:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .midi__snap--gold {
    border-color: var(--color-warning);
    background: color-mix(in oklab, var(--color-warning) 14%, var(--color-bg-2));
  }
  .midi__snap--active {
    border-color: var(--color-primary);
    background: color-mix(in oklab, var(--color-primary) 10%, var(--color-bg-2));
  }
  .midi__snap-row {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }
  .midi__snap-num {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
  }
  .midi__snap--gold .midi__snap-num {
    color: var(--color-warning);
  }
  .midi__snap--active .midi__snap-num {
    color: var(--color-primary);
  }
  .midi__snap-mode-pill {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
    background: var(--color-bg-3);
    padding: 1px 4px;
    border-radius: var(--radius-xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
  }
  .midi__snap-name {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }
  .midi__snap-name--empty {
    font-style: italic;
    color: var(--color-text-4);
  }
  .midi__snap--gold .midi__snap-name {
    color: var(--color-warning);
  }
  .midi__snap-binding {
    font-size: var(--font-size-caps-sm);
    color: var(--color-primary);
  }
  .midi__snap-binding--unbound {
    color: var(--color-text-4);
  }

  /* ---------------- Bindings table --------------------------------- */
  .midi__bindings {
    padding: var(--space-md) var(--space-lg);
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .midi__table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
  }
  .midi__col-target {
    width: 16%;
  }
  .midi__col-device {
    width: 16%;
  }
  .midi__col-ch,
  .midi__col-msg,
  .midi__col-num,
  .midi__col-min,
  .midi__col-max,
  .midi__col-thresh {
    width: 7%;
  }
  .midi__col-mode {
    width: 11%;
  }
  .midi__col-flag {
    width: 10%;
  }
  .midi__col-actions {
    width: 5%;
  }
  .midi__table thead th {
    text-align: left;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
    border-bottom: 1px solid var(--color-line-2);
    padding: var(--space-xs) var(--space-xs);
  }
  .midi__table tbody tr {
    border-top: 1px solid var(--color-line-1);
  }
  .midi__row--learning {
    background: color-mix(in oklab, var(--color-primary) 10%, transparent);
  }
  .midi__cell {
    padding: var(--space-xs) var(--space-xs);
    color: var(--color-text-1);
    vertical-align: middle;
  }
  .midi__cell--target {
    color: var(--color-primary);
    text-align: left;
    font-weight: var(--font-weight-label);
  }
  .midi__cell--flag {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }
  .midi__cell--actions {
    text-align: right;
  }
  .midi__select {
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    padding: 2px var(--space-xs);
  }
  .midi__select:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }
  .midi__saved-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-success);
    box-shadow: 0 0 calc(4px * var(--glow, 1)) var(--color-success);
    display: inline-block;
  }
  .midi__no-rows {
    padding: var(--space-md);
    color: var(--color-text-3);
    text-align: center;
  }

  /* ---------------- Live Scene Controls ---------------------------- */
  .midi__live {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }
  .midi__subhead {
    margin: 0;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-md);
    color: var(--color-text-2);
  }
  .midi__live-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--space-md);
  }
  .midi__group {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-2);
    border-top: 2px solid var(--group-tint);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .midi__group-head {
    padding: var(--space-xs) var(--space-md);
    background: var(--color-bg-3);
    border-bottom: 1px solid var(--color-line-2);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--group-tint);
  }
  .midi__group-body {
    padding: var(--space-md);
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--space-sm);
  }
  .midi__widget {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-xs);
  }
  .midi__widget-label {
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
  }

  /* widget-toggle */
  .midi__toggle {
    width: 22px;
    height: 36px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    position: relative;
    cursor: pointer;
    padding: 0;
  }
  .midi__toggle-knob {
    position: absolute;
    left: 2px;
    right: 2px;
    bottom: 2px;
    height: 14px;
    background: var(--color-bg-5);
    border-radius: 1px;
    transition: transform 120ms ease;
  }
  .midi__toggle--on .midi__toggle-knob {
    transform: translateY(-16px);
    background: var(--group-tint);
  }
  .midi__toggle:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  /* widget-fader */
  .midi__fader {
    width: 12px;
    height: 56px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    overflow: hidden;
    position: relative;
  }
  .midi__fader-fill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--group-tint);
    opacity: 0.95;
  }

  /* widget-bang */
  .midi__bang {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    color: var(--group-tint);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .midi__bang-glyph {
    font-size: 16px;
  }
  .midi__bang:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  /* widget-knob */
  .midi__knob {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    position: relative;
    transform: rotate(var(--knob-rot, -135deg));
  }
  .midi__knob-indicator {
    position: absolute;
    left: 50%;
    top: 4px;
    width: 2px;
    height: 10px;
    background: var(--group-tint);
    transform: translateX(-50%);
    border-radius: var(--radius-xs);
  }

  /* ---------------- Right column ----------------------------------- */
  .midi__side {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    min-width: 0;
  }

  .midi__panel--learn {
    border-color: var(--color-primary);
    box-shadow:
      0 0 0 1px var(--color-primary),
      0 0 calc(24px * var(--glow, 1)) color-mix(in oklab, var(--color-primary) 30%, transparent);
  }
  .midi__panel-head--learn {
    background: color-mix(in oklab, var(--color-primary) 10%, var(--color-bg-3));
  }
  .midi__panel-title--learn {
    color: var(--color-primary);
  }

  .midi__learn-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .midi__learn-target {
    font-size: var(--font-size-display-sm);
    color: var(--color-primary);
  }
  .midi__learn-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-md);
  }
  .midi__learn-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .midi__learn-text {
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-2);
  }

  /* ---------------- Waveform --------------------------------------- */
  .midi__waveform {
    height: 28px;
    margin-top: 4px;
    border-radius: var(--radius-xs);
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-1);
    position: relative;
    overflow: hidden;
  }
  .midi__waveform-svg {
    display: block;
    width: 100%;
    height: 100%;
  }
  .midi__waveform-grid {
    stroke: var(--color-line-1);
    stroke-width: 0.4;
    opacity: 0.6;
  }
  .midi__waveform-fill {
    fill: color-mix(in oklab, var(--color-primary) 18%, transparent);
  }
  .midi__waveform-line {
    fill: none;
    stroke: var(--color-primary);
    stroke-width: 1.4;
  }
  .midi__waveform-head {
    fill: var(--color-primary);
  }

  /* ---------------- Encoder pills ---------------------------------- */
  .midi__encoder-pills {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-xs);
  }

  /* ---------------- History strip ---------------------------------- */
  .midi__history {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    background: var(--color-bg-0);
    overflow: hidden;
  }
  .midi__history-row {
    display: grid;
    grid-template-columns: 72px 1fr 42px 46px;
    gap: var(--space-xs);
    padding: 3px var(--space-md);
    font-size: var(--font-size-caps-sm);
    align-items: center;
    border-top: 1px solid var(--color-line-1);
  }
  .midi__history-row:first-child {
    border-top: 0;
  }
  .midi__history-msg {
    color: var(--color-text-2);
  }
  .midi__history-row--head .midi__history-msg {
    color: var(--color-primary);
  }
  .midi__history-dev {
    color: var(--color-text-4);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .midi__history-hex {
    color: var(--color-text-2);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .midi__history-ago {
    color: var(--color-text-4);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .midi__learn-actions {
    display: flex;
    gap: var(--space-xs);
  }

  /* ---------------- BPM list --------------------------------------- */
  .midi__bpm-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .midi__bpm-row {
    display: grid;
    grid-template-columns: 14px 20px 1fr auto;
    gap: var(--space-sm);
    align-items: center;
    padding: var(--space-xs) var(--space-md);
    border-radius: var(--radius-xs);
    background: var(--color-bg-0);
    border: 1px solid var(--color-line-2);
  }
  .midi__bpm-row--active {
    background: color-mix(in oklab, var(--color-primary) 10%, var(--color-bg-0));
    border-color: var(--color-primary);
  }
  .midi__bpm-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-text-4);
  }
  .midi__bpm-row[data-state='active'] .midi__bpm-dot {
    background: var(--color-success);
    box-shadow: 0 0 calc(4px * var(--glow, 1)) var(--color-success);
  }
  .midi__bpm-row[data-state='standby'] .midi__bpm-dot {
    background: var(--color-warning);
  }
  .midi__bpm-rank {
    color: var(--color-text-3);
    font-size: var(--font-size-caps-sm);
  }
  .midi__bpm-name {
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-sm);
  }
  .midi__bpm-row--active .midi__bpm-name {
    color: var(--color-primary);
  }
  .midi__bpm-detail {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
  }
  .midi__bpm-actions {
    display: inline-flex;
    gap: 2px;
  }

  /* ---------------- MTC ------------------------------------------- */
  .midi__mtc {
    display: flex;
    align-items: baseline;
    gap: var(--space-sm);
  }
  .midi__mtc-tc {
    font-size: 22px;
    color: var(--color-primary);
    font-variant-numeric: tabular-nums;
  }
  .midi__mtc-fps {
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-3);
  }
  .midi__mtc-prose {
    margin: 0;
    font-size: var(--font-size-caps-sm);
    color: var(--color-text-2);
  }
  .midi__mtc-uniform {
    color: var(--color-info);
  }

  /* ---------------- Shared utility -------------------------------- */
  .mono {
    font-family: var(--font-family-mono);
  }
</style>
