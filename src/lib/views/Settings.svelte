<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
-->

<script lang="ts">
  // Wave-21 PR1 Settings expansion — extends the W14 PR3 Settings view
  // (which shipped Hotkeys + ThemeSwitcher only) with the four operator
  // bring-up sections flagged by the W20 feature-parity report
  // (`/tmp/feature-parity-report.md`):
  //
  //   1. Audio Device — input/output picker, sample rate, latency hint,
  //      channel layout. Visual stub; live cpal device enumeration is
  //      Phase-3+/4a `EV-INTEGRATION-MATRIX` scope.
  //   2. Graphics / Output — render resolution preset + custom WxH,
  //      target FPS, V-Sync toggle, HDR target, display assignment.
  //   3. Storage — Media folders + Custom scene folders + Backups path
  //      with retention; thumbnail / render-cache cleanup actions. Each
  //      browse button invokes the W21 PR1 `pick_folder` IPC stub.
  //   4. Folder-picker IPC — see `src/lib/ipc/picker.ts` and
  //      `app/shell/src/commands/picker.rs`. Phase-3+/4a swaps the
  //      placeholder for `tauri-plugin-dialog`'s native picker.
  //
  // The four new sections sit ABOVE the existing Hotkeys + ThemeSwitcher
  // sections — operator infrastructure is more frequently accessed than
  // hotkey reference. The General section retains the W14-banked
  // entitlement-locked toggles (OSC input adapter, Live Sources) since
  // they remain accurate stubs ahead of the W15+ entitlement plumbing.
  //
  // Per `feedback_wireframes_normative_for_ui_ux.md` the bundle JSX is
  // canonical: Audio / Graphics / Storage layouts mirror
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/settings.jsx`'s
  // `Section` + `Row(label, help, children)` grid. Capability-gated
  // rows (HDR target, custom resolution) carry `data-cap=...`
  // attributes for the future entitlement infra to discover.
  //
  // W21 PR1 visual-only stubs: each form field uses a local `$state`
  // variable so the controls feel live (selects update, toggles flip,
  // folder rows append on Browse) but no value is persisted across a
  // reload. Persistence + IPC wiring tracks the evidence-matrix slice.
  import { HOTKEYS } from '../data/hotkeys';
  import {
    ABLETON_LINK_PEER_FIXTURE,
    AUDIO_CHANNEL_LAYOUTS,
    AUDIO_CHANNEL_LEFT_OPTIONS,
    AUDIO_CHANNEL_RIGHT_OPTIONS,
    AUDIO_INPUT_DEVICES,
    AUDIO_INPUT_GAIN_DB_RANGE_DB,
    AUDIO_LATENCY_HINTS,
    AUDIO_METER_BARS,
    AUDIO_METER_LEVEL,
    AUDIO_METER_PEAK,
    AUDIO_OUTPUT_DEVICES,
    AUDIO_SAMPLE_RATES,
    BACKUP_RETENTION_OPTIONS,
    COLOR_SPACE_OPTIONS,
    DEFAULT_BACKUP_PATH,
    DEFAULT_MEDIA_FOLDERS,
    DEFAULT_SCENE_FOLDERS,
    DISPLAYS,
    FPS_TARGETS,
    HDR_TARGETS,
    MIDI_CLOCK_INPUT_DEVICES,
    MIDI_CLOCK_OUTPUT_DEVICES,
    MULTI_DISPLAY_MODE_OPTIONS,
    QUANTIZE_TRANSITION_OPTIONS,
    RENDER_SCALE_MAX,
    RENDER_SCALE_MIN,
    RENDER_SCALE_STEP,
    RESOLUTION_PRESETS,
    SCREEN_FITTING_OPTIONS,
    SENDER_NAME_DEFAULT,
    type ColorSpaceOption,
    type FolderEntry,
    type MultiDisplayModeOption,
    type QuantizeTransitionOption,
  } from '../data/settings-fixtures';
  import Icon from '../Icon.svelte';
  import { pickFolder } from '../ipc/picker';
  import { settingsStore } from '../stores/settings.svelte';
  import ThemeSwitcher from '../ThemeSwitcher.svelte';
  import { ControlGroup, Fader, Meter, OutputRoutingTile, TabSegment } from '../widgets';
  import SceneStill from '../widgets/SceneStill.svelte';

  // Wave 2.1 Lane C.3 — persist Audio / Graphics / General settings
  // via the `get_settings` / `set_settings` Tauri IPC handlers.
  // Storage / Live Sources / Tempo & Sync stay local-state-only at
  // this slice; they round-trip through the bundle-faithful fixture
  // defaults and don't survive restart yet. Theme persistence is
  // handled by the existing ThemeSwitcher (localStorage); the
  // settings document carries a `general.theme` field for future
  // unification but does NOT override the live ThemeSwitcher today.
  //
  // The store loads asynchronously on mount; until it resolves the
  // form fields render against the bundle-faithful defaults baked
  // into `settingsStore.snapshot` (which match the fixture defaults
  // verbatim, so the existing Settings view tests keep passing).
  $effect(() => {
    void settingsStore.load();
  });

  // Pull the audio + graphics + general sections through derived
  // reads so any store mutation reflects in the UI on next $effect
  // flush.
  const audioSnapshot = $derived(settingsStore.snapshot.audio);
  const graphicsSnapshot = $derived(settingsStore.snapshot.graphics);

  // W38 right-rail Diagnostics hardware data — bundle-faithful fixture
  // mirroring `views/settings.jsx` lines 187-194 (the right-column
  // Diagnostics panel rows). Frozen-frame; live values arrive with the
  // Phase-3+/4a `EV-OBSERVABILITY` runtime metric stream and the OSC view's
  // adapter-health surface. The strings here parallel the bundle's W20
  // wireframe-capture exactly so future operators see the same shape they
  // saw in the bundle preview.
  interface DiagnosticsRow {
    readonly key: string;
    readonly value: string;
  }
  const DIAGNOSTICS: readonly DiagnosticsRow[] = [
    { key: 'GPU', value: 'Apple M3 Pro · 42% · 16 GB' },
    { key: 'FPS / Frame', value: '61 · 16.4ms' },
    { key: 'Audio Latency', value: '5.3 ms' },
    { key: 'MIDI Devices', value: '2 connected' },
    { key: 'Syphon / Spout', value: 'Streaming → OBS' },
    { key: 'Ableton Link', value: '4 peers · 128.00 BPM' },
  ];

  /* ----- Audio Device — persisted via settingsStore -----
   * The form fields below read live from `settingsStore.snapshot.audio`
   * (via the `audioSnapshot` $derived above) and write back through
   * `settingsStore.update({ audio: { ... } })`. The store debounces
   * disk persistence so rapid slider drags collapse into one save. */
  const inputGainDb = $derived((audioSnapshot.input_gain - 0.5) * AUDIO_INPUT_GAIN_DB_RANGE_DB);
  const inputGainDbLabel = $derived(`${inputGainDb >= 0 ? '+' : ''}${inputGainDb.toFixed(1)} dB`);

  /* ----- Graphics / Output — persisted via settingsStore -----
   * Same pattern as Audio: read through `graphicsSnapshot` (the
   * $derived above); write through `settingsStore.update({ graphics })`.
   * W21 PR1 + W41 cohort fields all flow through the same seam. */
  const renderScaleLabel = $derived(`${graphicsSnapshot.render_scale.toFixed(2)}×`);

  /* ----- Live Sources (W42) -----
   * Bundle source: views/settings.jsx lines 115-124. Five boolean
   * toggles + one text input gating the Syphon/Spout (per-platform
   * sibling pair), NDI v2 input/output, Constant Output Resolution,
   * and Sender Name surfaces. Frozen-frame; live source enumeration
   * + the broadcast/receive IPC ride in alongside the Phase-3+/4a
   * integration crate. The `data-cap` attributes carry the bundle's
   * entitlement-gated capability ids (`io.syphon.*`, `io.ndi.*`,
   * `render.custom-resolution`) for the W15+ post-MVP SKU
   * infrastructure to consume without further markup changes. */
  let syphonInputEnabled = $state(true);
  let syphonOutputEnabled = $state(true);
  let ndiInputEnabled = $state(false);
  let ndiOutputEnabled = $state(true);
  let constantOutputResolution = $state(false);
  let senderName = $state(SENDER_NAME_DEFAULT);

  /* ----- Storage state ----- */
  let mediaFolders = $state<FolderEntry[]>(DEFAULT_MEDIA_FOLDERS.map((f) => ({ ...f })));
  let sceneFolders = $state<FolderEntry[]>(DEFAULT_SCENE_FOLDERS.map((f) => ({ ...f })));
  let backupPath = $state(DEFAULT_BACKUP_PATH);
  let backupRetention = $state(BACKUP_RETENTION_OPTIONS[1]?.id ?? 'retain-10');

  let folderIdCounter = DEFAULT_MEDIA_FOLDERS.length + DEFAULT_SCENE_FOLDERS.length + 1;

  async function browseMediaFolder() {
    const path = await pickFolder('Pick a Media folder');
    if (path) {
      mediaFolders = [...mediaFolders, { id: `mf-${folderIdCounter++}`, path }];
    }
  }

  async function browseSceneFolder() {
    const path = await pickFolder('Pick a Custom scene folder');
    if (path) {
      sceneFolders = [...sceneFolders, { id: `sf-${folderIdCounter++}`, path }];
    }
  }

  async function browseBackupFolder() {
    const path = await pickFolder('Pick a Backups folder');
    if (path) {
      backupPath = path;
    }
  }

  function removeMediaFolder(id: string) {
    mediaFolders = mediaFolders.filter((f) => f.id !== id);
  }

  function removeSceneFolder(id: string) {
    sceneFolders = sceneFolders.filter((f) => f.id !== id);
  }

  /* ----- Tempo & Sync state (W22 PR1) ----- */
  // Frozen-frame, no IPC: runtime wiring (Audio estimator, MIDI Clock,
  // Ableton Link peer discovery) lands in Phase-3+. The priority chain
  // is FIXED in W22 — drag-reorder + per-source health is Phase-3+ scope
  // per the M-10 BPM source resolver in
  // `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/midi.jsx`.

  type BpmSourceId = 'audio' | 'midiclock' | 'link' | 'manual';

  interface BpmSource {
    readonly id: BpmSourceId;
    readonly name: string;
    readonly detail: string;
  }

  const BPM_SOURCES: readonly BpmSource[] = [
    { id: 'audio', name: 'Audio estimate', detail: 'live spectrum onset detector' },
    { id: 'midiclock', name: 'MIDI Clock', detail: 'external clock from MIDI input' },
    { id: 'link', name: 'Ableton Link', detail: 'peer-to-peer LAN tempo sync' },
    { id: 'manual', name: 'Manual / Tap', detail: 'tap tempo or fixed BPM entry' },
  ];

  const ACTIVE_BPM_SOURCE: BpmSourceId = 'audio';

  let manualBpm = $state(120.0);
  let linkOffsetMs = $state(0);
  let audioBpmSensitivity = $state(60);

  let tapTimestamps = $state<number[]>([]);
  let tappedBpm = $state<number | null>(null);
  let tapResetTimer: ReturnType<typeof setTimeout> | null = null;

  const TAP_WINDOW = 4;
  const TAP_RESET_MS = 2000;

  function tapTempo() {
    const now = performance.now();
    const next = [...tapTimestamps, now].slice(-TAP_WINDOW);
    tapTimestamps = next;

    if (next.length >= 2) {
      let total = 0;
      for (let i = 1; i < next.length; i++) {
        total += (next[i] ?? 0) - (next[i - 1] ?? 0);
      }
      const meanInterval = total / (next.length - 1);
      tappedBpm = meanInterval > 0 ? 60000 / meanInterval : null;
    } else {
      tappedBpm = null;
    }

    if (tapResetTimer !== null) {
      clearTimeout(tapResetTimer);
    }
    tapResetTimer = setTimeout(() => {
      tapTimestamps = [];
      tappedBpm = null;
      tapResetTimer = null;
    }, TAP_RESET_MS);
  }

  function setManualBpm() {
    // Frozen-frame: clamp + round-trip through local state only.
    manualBpm = Math.min(300, Math.max(20, Math.round(manualBpm * 10) / 10));
  }

  /* ----- Tempo & Sync extensions (W40) -----
   * Bundle source: views/settings.jsx lines 52-77. Adds the five
   * settings.jsx Tempo & Sync rows that the W22 PR1 port left for
   * later: Ableton Link toggle + peer pill, Start/Stop Sync toggle,
   * MIDI Clock In + Out (each toggle + device select), and Quantize
   * Transitions tab-seg. Frozen-frame; live Link peer count + MIDI
   * clock device enumeration ride in alongside the Phase-3+ tempo
   * arbiter and the integration crate's MIDI/Link probes. */
  let linkEnabled = $state(true);
  let startStopSync = $state(true);
  let midiClockInEnabled = $state(false);
  let midiClockInDevice = $state(MIDI_CLOCK_INPUT_DEVICES[0]?.id ?? 'any');
  let midiClockOutEnabled = $state(true);
  let midiClockOutDevice = $state(MIDI_CLOCK_OUTPUT_DEVICES[0]?.id ?? 'iac-1');
  let quantize = $state<QuantizeTransitionOption['id']>('bar');
</script>

<section class="settings">
  <div class="settings__main">
    <header class="settings__header">
      <h1>settings</h1>
      <p class="settings__intro">
        Operator preferences, devices, output, storage, hotkey reference, and about.
        Wireframe-aligned with the bundle's <code>views/settings.jsx</code>; W21 PR1 ships the
        visual-only Audio / Graphics / Storage scaffolding plus the <code>pick_folder</code> IPC contract.
        Live device enumeration + the native folder dialog land in Phase-3+/4a. W38 adds the right-rail
        Preview + Diagnostics panel mirroring the bundle's two-column layout. W39 expands the Audio Device
        section with Channel Routing, Input Gain, and a Monitoring meter.
      </p>
    </header>

    <article class="settings__section">
      <h2>Audio Device</h2>
      <div class="settings__form">
        <div class="settings__field">
          <label for="audio-input" class="settings__label">Input Device</label>
          <select
            id="audio-input"
            class="settings__select"
            value={audioSnapshot.input_device}
            onchange={(e) =>
              settingsStore.update({ audio: { input_device: e.currentTarget.value } })}
          >
            {#each AUDIO_INPUT_DEVICES as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field">
          <label for="audio-output" class="settings__label">Output Device</label>
          <select
            id="audio-output"
            class="settings__select"
            value={audioSnapshot.output_device}
            onchange={(e) =>
              settingsStore.update({ audio: { output_device: e.currentTarget.value } })}
          >
            {#each AUDIO_OUTPUT_DEVICES as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field">
          <label for="sample-rate" class="settings__label">Sample Rate</label>
          <select
            id="sample-rate"
            class="settings__select"
            value={String(audioSnapshot.sample_rate_hz)}
            onchange={(e) =>
              settingsStore.update({
                audio: { sample_rate_hz: Number.parseInt(e.currentTarget.value, 10) },
              })}
          >
            {#each AUDIO_SAMPLE_RATES as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field">
          <label for="latency-hint" class="settings__label">Latency Hint</label>
          <select
            id="latency-hint"
            class="settings__select"
            value={audioSnapshot.latency_hint}
            onchange={(e) =>
              settingsStore.update({ audio: { latency_hint: e.currentTarget.value } })}
          >
            {#each AUDIO_LATENCY_HINTS as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field">
          <label for="channel-layout" class="settings__label">Channel Layout</label>
          <select
            id="channel-layout"
            class="settings__select"
            value={audioSnapshot.channel_layout}
            onchange={(e) =>
              settingsStore.update({ audio: { channel_layout: e.currentTarget.value } })}
          >
            {#each AUDIO_CHANNEL_LAYOUTS as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field">
          <span class="settings__label">Channel Routing</span>
          <div class="settings__field-row">
            <label class="settings__a11y-only" for="channel-left">Left channel</label>
            <select
              id="channel-left"
              class="settings__select settings__select--compact"
              value={audioSnapshot.channel_left}
              onchange={(e) =>
                settingsStore.update({ audio: { channel_left: e.currentTarget.value } })}
            >
              {#each AUDIO_CHANNEL_LEFT_OPTIONS as opt (opt.id)}
                <option value={opt.id}>{opt.label}</option>
              {/each}
            </select>
            <label class="settings__a11y-only" for="channel-right">Right channel</label>
            <select
              id="channel-right"
              class="settings__select settings__select--compact"
              value={audioSnapshot.channel_right}
              onchange={(e) =>
                settingsStore.update({ audio: { channel_right: e.currentTarget.value } })}
            >
              {#each AUDIO_CHANNEL_RIGHT_OPTIONS as opt (opt.id)}
                <option value={opt.id}>{opt.label}</option>
              {/each}
            </select>
            <span class="settings__hint settings__hint--inline">stereo</span>
          </div>
        </div>
        <div class="settings__field">
          <label for="input-gain" class="settings__label">Input Gain</label>
          <div class="settings__field-row settings__field-row--gain">
            <input
              id="input-gain"
              type="range"
              class="settings__range"
              min="0"
              max="1"
              step="0.01"
              value={audioSnapshot.input_gain}
              oninput={(e) =>
                settingsStore.update({
                  audio: { input_gain: Number.parseFloat(e.currentTarget.value) },
                })}
              aria-valuetext={inputGainDbLabel}
            />
            <span class="settings__bpm-readout settings__bpm-readout--gain">
              {inputGainDbLabel}
            </span>
          </div>
        </div>
        <div class="settings__field">
          <span class="settings__label">Monitoring</span>
          <div class="settings__field-row settings__field-row--meter">
            <Meter
              level={AUDIO_METER_LEVEL}
              peak={AUDIO_METER_PEAK}
              bars={AUDIO_METER_BARS}
              ariaLabel="Audio input monitor"
            />
          </div>
        </div>
      </div>
      <p class="settings__hint">
        Visual stub — live cpal device enumeration arrives in Phase-3+/4a per
        <code>EV-INTEGRATION-MATRIX</code>. The Monitoring meter is frozen-frame; live RMS /
        peak-hold ride in alongside the audio capture loop per <code>EV-OBSERVABILITY</code>.
      </p>
    </article>

    <article class="settings__section">
      <h2>Graphics / Output</h2>
      <div class="settings__form">
        <div class="settings__field">
          <label for="resolution-preset" class="settings__label">Render Resolution</label>
          <select
            id="resolution-preset"
            class="settings__select"
            value={graphicsSnapshot.resolution_preset}
            onchange={(e) =>
              settingsStore.update({
                graphics: { resolution_preset: e.currentTarget.value },
              })}
            disabled={graphicsSnapshot.custom_resolution_enabled}
          >
            {#each RESOLUTION_PRESETS as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field" data-cap="render.custom-resolution">
          <span class="settings__label">Custom Resolution</span>
          <div class="settings__field-row">
            <button
              type="button"
              class="settings__pill-toggle"
              class:on={graphicsSnapshot.custom_resolution_enabled}
              aria-pressed={graphicsSnapshot.custom_resolution_enabled}
              aria-label="Custom resolution"
              onclick={() =>
                settingsStore.update({
                  graphics: {
                    custom_resolution_enabled: !graphicsSnapshot.custom_resolution_enabled,
                  },
                })}
            >
              <span class="settings__pill-thumb"></span>
            </button>
            <label for="custom-width" class="settings__sublabel">W</label>
            <input
              id="custom-width"
              type="number"
              class="settings__input settings__input--num"
              min="320"
              max="7680"
              step="2"
              value={graphicsSnapshot.custom_width}
              oninput={(e) =>
                settingsStore.update({
                  graphics: {
                    custom_width: Number.parseInt(e.currentTarget.value, 10) || 0,
                  },
                })}
              disabled={!graphicsSnapshot.custom_resolution_enabled}
            />
            <span class="settings__times">×</span>
            <label for="custom-height" class="settings__sublabel">H</label>
            <input
              id="custom-height"
              type="number"
              class="settings__input settings__input--num"
              min="240"
              max="4320"
              step="2"
              value={graphicsSnapshot.custom_height}
              oninput={(e) =>
                settingsStore.update({
                  graphics: {
                    custom_height: Number.parseInt(e.currentTarget.value, 10) || 0,
                  },
                })}
              disabled={!graphicsSnapshot.custom_resolution_enabled}
            />
          </div>
        </div>
        <div class="settings__field">
          <label for="fps-target" class="settings__label">Target FPS</label>
          <select
            id="fps-target"
            class="settings__select"
            value={graphicsSnapshot.fps_target}
            onchange={(e) =>
              settingsStore.update({ graphics: { fps_target: e.currentTarget.value } })}
          >
            {#each FPS_TARGETS as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field">
          <span class="settings__label">V-Sync</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={graphicsSnapshot.vsync}
            aria-pressed={graphicsSnapshot.vsync}
            aria-label="V-Sync"
            onclick={() => settingsStore.update({ graphics: { vsync: !graphicsSnapshot.vsync } })}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field" data-cap="render.hdr">
          <label for="hdr-target" class="settings__label">HDR Target</label>
          <select
            id="hdr-target"
            class="settings__select"
            value={graphicsSnapshot.hdr_target}
            onchange={(e) =>
              settingsStore.update({ graphics: { hdr_target: e.currentTarget.value } })}
          >
            {#each HDR_TARGETS as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field">
          <label for="display" class="settings__label">Display</label>
          <select
            id="display"
            class="settings__select"
            value={graphicsSnapshot.display}
            onchange={(e) => settingsStore.update({ graphics: { display: e.currentTarget.value } })}
          >
            {#each DISPLAYS as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field">
          <span class="settings__label">Color Space</span>
          <div class="settings__field-row">
            <TabSegment
              options={COLOR_SPACE_OPTIONS}
              value={graphicsSnapshot.color_space as ColorSpaceOption['id']}
              onChange={(id) =>
                settingsStore.update({
                  graphics: { color_space: id as ColorSpaceOption['id'] },
                })}
              ariaLabel="Color space"
            />
            <span class="settings__hint settings__hint--inline">
              gamma / primaries for the output surface
            </span>
          </div>
        </div>
        <div class="settings__field">
          <span class="settings__label">Multi-Display Mode</span>
          <div class="settings__field-row">
            <TabSegment
              options={MULTI_DISPLAY_MODE_OPTIONS}
              value={graphicsSnapshot.multi_display_mode as MultiDisplayModeOption['id']}
              onChange={(id) =>
                settingsStore.update({
                  graphics: { multi_display_mode: id as MultiDisplayModeOption['id'] },
                })}
              ariaLabel="Multi-display mode"
            />
            <span class="settings__hint settings__hint--inline">
              spanning treats outputs as one canvas; mirror duplicates
            </span>
          </div>
        </div>
        <div class="settings__field">
          <label for="render-scale" class="settings__label">Render Scale</label>
          <div class="settings__field-row settings__field-row--gain">
            <input
              id="render-scale"
              type="range"
              class="settings__range"
              min={RENDER_SCALE_MIN}
              max={RENDER_SCALE_MAX}
              step={RENDER_SCALE_STEP}
              value={graphicsSnapshot.render_scale}
              oninput={(e) =>
                settingsStore.update({
                  graphics: { render_scale: Number.parseFloat(e.currentTarget.value) },
                })}
              aria-valuetext={renderScaleLabel}
            />
            <span class="settings__bpm-readout settings__bpm-readout--gain">
              {renderScaleLabel}
            </span>
          </div>
        </div>
        <div class="settings__field">
          <span class="settings__label">Screen Fitting</span>
          <select
            id="screen-fitting"
            class="settings__select"
            value={graphicsSnapshot.screen_fitting}
            onchange={(e) =>
              settingsStore.update({
                graphics: { screen_fitting: e.currentTarget.value },
              })}
            aria-label="Screen fitting"
          >
            {#each SCREEN_FITTING_OPTIONS as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
        </div>
        <div class="settings__field">
          <span class="settings__label">Visualizer Window</span>
          <div class="settings__field-row">
            <button type="button" class="settings__btn settings__btn--ghost" disabled
              >Show resizable window</button
            >
            <button type="button" class="settings__btn settings__btn--ghost" disabled
              >Toggle fullscreen</button
            >
            <span class="settings__hint settings__hint--inline">
              window-management IPC arrives in Phase-3+/4a
            </span>
          </div>
        </div>
        <div class="settings__field">
          <span class="settings__label">FPS Slider</span>
          <div class="settings__field-row">
            <button
              type="button"
              class="settings__pill-toggle"
              class:on={graphicsSnapshot.fps_slider_enabled}
              aria-pressed={graphicsSnapshot.fps_slider_enabled}
              aria-label="FPS slider"
              onclick={() =>
                settingsStore.update({
                  graphics: {
                    fps_slider_enabled: !graphicsSnapshot.fps_slider_enabled,
                  },
                })}
            >
              <span class="settings__pill-thumb"></span>
            </button>
            <span class="settings__hint settings__hint--inline">
              show a draggable FPS cap handle in the topbar
            </span>
          </div>
        </div>
      </div>
      <p class="settings__hint">
        Capability-gated rows (custom resolution, HDR) carry <code>data-cap</code> attributes for the
        W15+ entitlement infrastructure to discover.
      </p>

      <div class="settings__output-routing">
        <span class="settings__label">Active output routing</span>
        <OutputRoutingTile />
      </div>
      <p class="settings__hint">
        Frozen-frame status panel — display + Syphon/Spout rows mirror the bundle's
        <code>OutputRoutingTile</code>. Live adapter probing arrives in Phase-3+/4a per
        <code>EV-INTEGRATION-MATRIX</code>; capability hints (<code>io.syphon.output</code>,
        <code>io.spout.output</code>, <code>io.ndi.output</code>) ride along as
        <code>data-cap</code> attributes for the W15+ entitlement infra.
      </p>
    </article>

    <article class="settings__section">
      <h2>Live Sources</h2>
      <div class="settings__form">
        <div class="settings__field" data-cap="io.syphon.input">
          <span class="settings__label">Spout / Syphon Input</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={syphonInputEnabled}
            aria-pressed={syphonInputEnabled}
            aria-label="Spout / Syphon Input"
            onclick={() => (syphonInputEnabled = !syphonInputEnabled)}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field" data-cap="io.syphon.output">
          <span class="settings__label">Spout / Syphon Output</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={syphonOutputEnabled}
            aria-pressed={syphonOutputEnabled}
            aria-label="Spout / Syphon Output"
            onclick={() => (syphonOutputEnabled = !syphonOutputEnabled)}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field" data-cap="io.ndi.input">
          <span class="settings__label">NDI Input</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={ndiInputEnabled}
            aria-pressed={ndiInputEnabled}
            aria-label="NDI Input"
            onclick={() => (ndiInputEnabled = !ndiInputEnabled)}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field" data-cap="io.ndi.output">
          <span class="settings__label">NDI Output</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={ndiOutputEnabled}
            aria-pressed={ndiOutputEnabled}
            aria-label="NDI Output"
            onclick={() => (ndiOutputEnabled = !ndiOutputEnabled)}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field" data-cap="render.custom-resolution">
          <span class="settings__label">Constant Output Resolution</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={constantOutputResolution}
            aria-pressed={constantOutputResolution}
            aria-label="Constant Output Resolution"
            onclick={() => (constantOutputResolution = !constantOutputResolution)}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field">
          <label for="sender-name" class="settings__label">Sender Name</label>
          <input
            id="sender-name"
            type="text"
            class="settings__input settings__input--sender"
            bind:value={senderName}
            maxlength="32"
          />
        </div>
      </div>
      <p class="settings__hint">
        Frozen-frame; live source enumeration and the broadcast / receive IPC arrive in Phase-3+/4a
        per <code>EV-INTEGRATION-MATRIX</code>. <code>data-cap</code> attributes (<code
          >io.syphon.input</code
        >, <code>io.syphon.output</code>, <code>io.ndi.input</code>,
        <code>io.ndi.output</code>, <code>render.custom-resolution</code>) ride along for the W15+
        entitlement infra to discover without further markup changes.
      </p>
    </article>

    <article class="settings__section">
      <h2>Storage</h2>

      <div class="settings__storage-group">
        <div class="settings__storage-head">
          <h3 class="settings__storage-title">Media folders</h3>
          <button
            type="button"
            class="settings__btn settings__btn--ghost"
            onclick={browseMediaFolder}
            aria-label="Add media folder"
          >
            <Icon name="plus" size={12} /> Add Folder
          </button>
        </div>
        {#if mediaFolders.length === 0}
          <p class="settings__hint">No media folders configured.</p>
        {:else}
          <ul class="settings__folder-list">
            {#each mediaFolders as folder (folder.id)}
              <li class="settings__folder-row">
                <span class="settings__folder-icon"><Icon name="folder" size={14} /></span>
                <span class="settings__folder-path">{folder.path}</span>
                <button
                  type="button"
                  class="settings__btn settings__btn--ghost"
                  onclick={browseMediaFolder}
                  aria-label="Browse media folder"
                >
                  Browse
                </button>
                <button
                  type="button"
                  class="settings__btn settings__btn--danger"
                  onclick={() => removeMediaFolder(folder.id)}
                  aria-label="Remove media folder"
                >
                  <Icon name="trash" size={12} /> Remove
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="settings__storage-group">
        <div class="settings__storage-head">
          <h3 class="settings__storage-title">Custom scene folders</h3>
          <button
            type="button"
            class="settings__btn settings__btn--ghost"
            onclick={browseSceneFolder}
            aria-label="Add scene folder"
          >
            <Icon name="plus" size={12} /> Add Folder
          </button>
        </div>
        {#if sceneFolders.length === 0}
          <p class="settings__hint">No custom scene folders configured.</p>
        {:else}
          <ul class="settings__folder-list">
            {#each sceneFolders as folder (folder.id)}
              <li class="settings__folder-row">
                <span class="settings__folder-icon"><Icon name="folder" size={14} /></span>
                <span class="settings__folder-path">{folder.path}</span>
                <button
                  type="button"
                  class="settings__btn settings__btn--ghost"
                  onclick={browseSceneFolder}
                  aria-label="Browse scene folder"
                >
                  Browse
                </button>
                <button
                  type="button"
                  class="settings__btn settings__btn--danger"
                  onclick={() => removeSceneFolder(folder.id)}
                  aria-label="Remove scene folder"
                >
                  <Icon name="trash" size={12} /> Remove
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="settings__storage-group">
        <div class="settings__storage-head">
          <h3 class="settings__storage-title">Backups</h3>
        </div>
        <div class="settings__form">
          <div class="settings__field">
            <span class="settings__label">Path</span>
            <div class="settings__field-row">
              <span class="settings__folder-path settings__folder-path--inline">{backupPath}</span>
              <button
                type="button"
                class="settings__btn settings__btn--ghost"
                onclick={browseBackupFolder}
                aria-label="Browse backup folder"
              >
                <Icon name="folder" size={12} /> Browse
              </button>
            </div>
          </div>
          <div class="settings__field">
            <label for="backup-retention" class="settings__label">Retention</label>
            <select id="backup-retention" class="settings__select" bind:value={backupRetention}>
              {#each BACKUP_RETENTION_OPTIONS as opt (opt.id)}
                <option value={opt.id}>{opt.label}</option>
              {/each}
            </select>
          </div>
        </div>
      </div>

      <div class="settings__storage-group">
        <div class="settings__storage-head">
          <h3 class="settings__storage-title">Cache</h3>
        </div>
        <div class="settings__row">
          <button type="button" class="settings__btn settings__btn--ghost">
            <Icon name="refresh" size={12} /> Clear thumbnails
          </button>
          <button type="button" class="settings__btn settings__btn--ghost">
            <Icon name="refresh" size={12} /> Clear render cache
          </button>
        </div>
        <p class="settings__hint">
          Cache cleanup actions are visual-only in W21 PR1; thumbnail + render-cache eviction IPC
          arrives alongside the Phase-3+/4a media adapter.
        </p>
      </div>
    </article>

    <article class="settings__section">
      <h2>Tempo &amp; Sync</h2>

      <div class="settings__storage-group">
        <div class="settings__storage-head">
          <h3 class="settings__storage-title">BPM source priority</h3>
        </div>
        <p class="settings__hint settings__hint--inline">
          First active source wins. Order is fixed in W22; drag-reorder + live source health arrives
          in Phase-3+ per the M-10 BPM resolver.
        </p>
        <ol class="settings__bpm-chain">
          {#each BPM_SOURCES as src, idx (src.id)}
            <li class="settings__bpm-row" class:active={src.id === ACTIVE_BPM_SOURCE}>
              <span class="settings__bpm-rank">{idx + 1}</span>
              <span
                class="settings__bpm-dot"
                aria-hidden="true"
                data-state={src.id === ACTIVE_BPM_SOURCE ? 'active' : 'idle'}
              ></span>
              <span class="settings__bpm-name">{src.name}</span>
              <span class="settings__bpm-detail">{src.detail}</span>
              {#if src.id === ACTIVE_BPM_SOURCE}
                <span class="settings__bpm-badge">active</span>
              {/if}
            </li>
          {/each}
        </ol>
      </div>

      <div class="settings__form">
        <div class="settings__field">
          <span class="settings__label">Tap tempo</span>
          <div class="settings__field-row">
            <button
              type="button"
              class="settings__btn settings__btn--ghost"
              onclick={tapTempo}
              aria-label="Tap tempo"
            >
              <Icon name="bang" size={12} /> Tap
            </button>
            <span class="settings__bpm-readout">
              {#if tappedBpm !== null}
                {tappedBpm.toFixed(1)} <span class="settings__bpm-unit">BPM</span>
              {:else}
                <span class="settings__muted">— BPM</span>
              {/if}
            </span>
            <span class="settings__hint settings__hint--inline">
              mean of last {TAP_WINDOW} taps; resets after 2s of silence
            </span>
          </div>
        </div>
        <div class="settings__field">
          <label for="manual-bpm" class="settings__label">Manual BPM</label>
          <div class="settings__field-row">
            <input
              id="manual-bpm"
              type="number"
              class="settings__input settings__input--num"
              min="20"
              max="300"
              step="0.1"
              bind:value={manualBpm}
            />
            <button type="button" class="settings__btn settings__btn--ghost" onclick={setManualBpm}>
              Set
            </button>
          </div>
        </div>
        <div class="settings__field">
          <label for="link-offset" class="settings__label">Link offset (ms)</label>
          <div class="settings__field-row">
            <input
              id="link-offset"
              type="number"
              class="settings__input settings__input--num"
              min="-200"
              max="200"
              step="1"
              bind:value={linkOffsetMs}
            />
            <span class="settings__hint settings__hint--inline">
              compensates for downstream-link latency, signed
            </span>
          </div>
        </div>
        <div class="settings__field">
          <span class="settings__label">Ableton Link</span>
          <div class="settings__field-row">
            <button
              type="button"
              class="settings__pill-toggle"
              class:on={linkEnabled}
              aria-pressed={linkEnabled}
              aria-label="Ableton Link"
              onclick={() => (linkEnabled = !linkEnabled)}
            >
              <span class="settings__pill-thumb"></span>
            </button>
            <span class="settings__link-pill" data-state={linkEnabled ? 'active' : 'idle'}>
              <span class="settings__link-dot" aria-hidden="true"></span>
              {ABLETON_LINK_PEER_FIXTURE.count} peers · {ABLETON_LINK_PEER_FIXTURE.bpm.toFixed(2)}
              BPM
            </span>
            <span class="settings__hint settings__hint--inline">
              sync BPM + phase with Link-enabled apps on the LAN
            </span>
          </div>
        </div>
        <div class="settings__field">
          <span class="settings__label">Start / Stop Sync</span>
          <div class="settings__field-row">
            <button
              type="button"
              class="settings__pill-toggle"
              class:on={startStopSync}
              aria-pressed={startStopSync}
              aria-label="Start Stop Sync"
              onclick={() => (startStopSync = !startStopSync)}
            >
              <span class="settings__pill-thumb"></span>
            </button>
            <span class="settings__hint settings__hint--inline">
              follow transport start / stop messages across Link peers
            </span>
          </div>
        </div>
        <div class="settings__field">
          <label for="midi-clock-in-device" class="settings__label">MIDI Clock Input</label>
          <div class="settings__field-row">
            <button
              type="button"
              class="settings__pill-toggle"
              class:on={midiClockInEnabled}
              aria-pressed={midiClockInEnabled}
              aria-label="MIDI Clock Input"
              onclick={() => (midiClockInEnabled = !midiClockInEnabled)}
            >
              <span class="settings__pill-thumb"></span>
            </button>
            <select
              id="midi-clock-in-device"
              class="settings__select settings__select--compact"
              bind:value={midiClockInDevice}
              disabled={!midiClockInEnabled}
            >
              {#each MIDI_CLOCK_INPUT_DEVICES as opt (opt.id)}
                <option value={opt.id}>{opt.label}</option>
              {/each}
            </select>
          </div>
        </div>
        <div class="settings__field">
          <label for="midi-clock-out-device" class="settings__label">MIDI Clock Output</label>
          <div class="settings__field-row">
            <button
              type="button"
              class="settings__pill-toggle"
              class:on={midiClockOutEnabled}
              aria-pressed={midiClockOutEnabled}
              aria-label="MIDI Clock Output"
              onclick={() => (midiClockOutEnabled = !midiClockOutEnabled)}
            >
              <span class="settings__pill-thumb"></span>
            </button>
            <select
              id="midi-clock-out-device"
              class="settings__select settings__select--compact"
              bind:value={midiClockOutDevice}
              disabled={!midiClockOutEnabled}
            >
              {#each MIDI_CLOCK_OUTPUT_DEVICES as opt (opt.id)}
                <option value={opt.id}>{opt.label}</option>
              {/each}
            </select>
          </div>
        </div>
        <div class="settings__field">
          <span class="settings__label">Quantize Transitions</span>
          <div class="settings__field-row">
            <TabSegment
              options={QUANTIZE_TRANSITION_OPTIONS}
              value={quantize}
              onChange={(id) => (quantize = id as QuantizeTransitionOption['id'])}
              ariaLabel="Quantize transitions"
            />
            <span class="settings__hint settings__hint--inline">
              snap scene changes to beat, bar, or phrase
            </span>
          </div>
        </div>
      </div>

      <div class="settings__bpm-sensitivity">
        <ControlGroup name="audio" tint="var(--color-cat-motion)">
          <Fader value={audioBpmSensitivity / 100} label="BPM sense" />
        </ControlGroup>
        <div class="settings__bpm-sensitivity-meta">
          <span class="settings__label">Audio BPM sensitivity</span>
          <span class="settings__bpm-readout">
            {audioBpmSensitivity}<span class="settings__bpm-unit">%</span>
          </span>
          <p class="settings__hint">
            Tuning weight applied to the audio onset estimator before it competes in the priority
            chain. Tinted via <code>--tint</code> per the W18 ControlGroup cascade contract.
          </p>
        </div>
      </div>

      <p class="settings__hint">
        Frozen-frame stub — no IPC dispatch. Runtime tempo arbiter, drag-reorder of the priority
        chain, and live Link peer count land in Phase-3+ alongside the audio estimator.
      </p>
    </article>

    <article class="settings__section">
      <h2>General</h2>
      <div class="settings__row">
        <ThemeSwitcher />
        <p class="settings__hint">
          Three-theme cascade per <code>tokens.css</code>. Selection is session-scoped (no
          persistence yet); resets to <em>pulse</em> on reload.
        </p>
      </div>
      <div class="settings__row settings__row--locked" data-cap="io.osc.input">
        <label class="settings__toggle">
          <input
            type="checkbox"
            disabled
            checked
            title="Entitlement-gated; arrives in W15+ alongside the OSC adapter UI"
          />
          <span>OSC input adapter</span>
        </label>
        <p class="settings__hint">Locked — entitlement infrastructure lands in W15+.</p>
      </div>
      <div class="settings__row settings__row--locked" data-cap="io.camera">
        <label class="settings__toggle">
          <input
            type="checkbox"
            disabled
            title="Entitlement-gated; arrives in W15+ alongside the Media library"
          />
          <span>Live Sources (camera capture)</span>
        </label>
        <p class="settings__hint">Locked — Media adapter integration lands in W15+.</p>
      </div>
    </article>

    <article class="settings__section">
      <h2>Hotkeys</h2>
      <p class="settings__hint">
        Read-only reference. The hotkey overlay (<kbd>?</kbd>) shows the same data in modal form.
        Rebinding is V2 scope per <code>docs/42-operator-hotkey-map.md</code> §V1.
      </p>
      {#each HOTKEYS as group (group.title)}
        <details class="settings__hotkey-group" open>
          <summary>{group.title}</summary>
          {#if group.note}
            <p class="settings__hint settings__hint--inline">{group.note}</p>
          {/if}
          <table class="settings__hotkey-table">
            <thead>
              <tr>
                <th scope="col">keys</th>
                <th scope="col">action</th>
                <th scope="col">osc</th>
                <th scope="col">cap</th>
              </tr>
            </thead>
            <tbody>
              {#each group.rows as row, i (i)}
                <tr>
                  <td class="settings__hotkey-keys">
                    {#each row.keys as k, j (j)}
                      {#if j > 0}<span class="settings__sep">/</span>{/if}<kbd>{k}</kbd>
                    {/each}
                  </td>
                  <td>{row.action}</td>
                  <td>
                    {#if row.osc}
                      <code>{row.osc}</code>
                    {:else}
                      <span class="settings__muted">—</span>
                    {/if}
                  </td>
                  <td>
                    {#if row.cap}
                      <span class="settings__cap" data-cap={row.cap}>{row.cap}</span>
                    {:else}
                      <span class="settings__muted">—</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </details>
      {/each}
    </article>

    <article class="settings__section">
      <h2>About</h2>
      <dl class="settings__about">
        <dt>version</dt>
        <dd>v0.0.0</dd>
        <dt>phase</dt>
        <dd>Phase 1 EXIT (6/6 evidence gates Pass)</dd>
        <dt>license</dt>
        <dd><code>LicenseRef-Chromaticity-Proprietary</code></dd>
        <dt>repo</dt>
        <dd><code>PhantasticUniverse/chromaticity</code></dd>
        <dt>toolkit</dt>
        <dd>Tauri 2 + Svelte 5 + Rust 2024 (per ADR-018)</dd>
      </dl>
    </article>
  </div>

  <aside class="settings__rail" aria-label="Settings preview and diagnostics">
    <div class="settings__preview">
      <div class="settings__preview-frame">
        <SceneStill
          kind="kaleido"
          seed={7}
          width={320}
          height={180}
          ariaLabel="Settings preview thumbnail"
        />
      </div>
      <span class="settings__preview-label">Monitor</span>
    </div>

    <article class="settings__section settings__section--diag">
      <h2>Diagnostics</h2>
      <dl class="settings__diag-data">
        {#each DIAGNOSTICS as row (row.key)}
          <div class="settings__diag-row">
            <dt class="settings__diag-key">{row.key}</dt>
            <dd class="settings__diag-value">{row.value}</dd>
          </div>
        {/each}
      </dl>
      <div class="settings__row">
        <button
          type="button"
          disabled
          title="Generates a support bundle (logs + scene state + adapter health) in W15+"
        >
          <Icon name="save" size={12} /> Generate Support Bundle
        </button>
        <button type="button" disabled title="Tail-style log viewer arrives in W15+">
          View Logs
        </button>
      </div>
      <p class="settings__hint">
        Hardware values are frozen-frame fixtures mirroring the bundle's <code
          >views/settings.jsx</code
        >
        Diagnostics rows. Live values stream in via the Phase-3+/4a runtime
        <code>EV-OBSERVABILITY</code> metric pipeline; support-bundle IPC and log streaming arrive in
        W15+ alongside the OSC view's adapter health surface.
      </p>
    </article>
  </aside>
</section>

<style>
  /* W38 layout: two-column grid (main + rail) on viewports wide enough
   * for the bundle's `1fr / 380px` proportions; collapses to single
   * column on narrow viewports so mobile / split-screen use stays
   * readable. The rail width stays fixed at 360px (slightly tighter
   * than the bundle's 380px to leave breathing room around the
   * preview canvas at typical desktop widths). */
  .settings {
    max-width: 1280px;
    margin: 0 auto;
    padding: var(--space-2xl) var(--space-xl);
    color: var(--color-text-0);
    display: grid;
    grid-template-columns: minmax(0, 1fr) 360px;
    gap: var(--space-xl);
    align-items: start;
  }
  .settings__main {
    min-width: 0;
  }
  @media (max-width: 960px) {
    .settings {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .settings__header {
    margin-bottom: var(--space-2xl);
  }

  .settings__header h1 {
    margin: 0 0 var(--space-sm) 0;
    font-size: var(--font-size-display-md);
    font-weight: 700;
  }

  .settings__intro {
    color: var(--color-text-2);
    font-size: var(--font-size-body-md);
    line-height: 1.5;
    margin: 0;
  }

  .settings__section {
    border-top: 1px solid var(--color-line-1);
    padding-top: var(--space-xl);
    margin-bottom: var(--space-2xl);
  }

  .settings__section h2 {
    margin: 0 0 var(--space-md) 0;
    font-size: var(--font-size-display-sm);
    font-weight: 600;
    color: var(--color-text-0);
  }

  .settings__row {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    margin-bottom: var(--space-md);
    flex-wrap: wrap;
  }

  .settings__row--locked {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .settings__row--locked .settings__toggle {
    cursor: not-allowed;
  }

  .settings__toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
  }

  .settings__hint {
    color: var(--color-text-2);
    font-size: var(--font-size-body-sm);
    line-height: 1.4;
    margin: 0;
  }

  .settings__hint--inline {
    margin: var(--space-xs) 0 var(--space-sm) 0;
  }

  /* ----- Form scaffolding (Audio / Graphics / Storage) ----- */

  .settings__form {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: var(--space-md) var(--space-lg);
    align-items: center;
    margin-bottom: var(--space-md);
  }

  .settings__field {
    display: contents;
  }

  .settings__label {
    color: var(--color-text-2);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 500;
  }

  .settings__sublabel {
    color: var(--color-text-3);
    font-size: var(--font-size-caps-md);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .settings__field-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .settings__select,
  .settings__input {
    background: var(--color-bg-2);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
    font-size: var(--font-size-body-sm);
    font-family: var(--font-family-sans);
    min-width: 220px;
  }

  .settings__select:focus,
  .settings__input:focus {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .settings__select:disabled,
  .settings__input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .settings__input--num {
    min-width: 0;
    width: 80px;
    font-family: var(--font-family-mono);
  }

  /* ----- W39 Audio routing / gain / monitoring ----- */

  .settings__select--compact {
    min-width: 88px;
    font-family: var(--font-family-mono);
  }

  .settings__a11y-only {
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

  .settings__field-row--gain {
    flex-wrap: nowrap;
    max-width: 420px;
    width: 100%;
  }

  .settings__field-row--meter {
    flex: 1;
    max-width: 420px;
  }

  .settings__range {
    flex: 1;
    min-width: 120px;
    accent-color: var(--color-primary);
    cursor: pointer;
    background: transparent;
  }

  .settings__range:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 4px;
    border-radius: var(--radius-xs);
  }

  .settings__bpm-readout--gain {
    font-size: var(--font-size-body-sm);
    min-width: 72px;
    text-align: right;
  }

  /* ----- W42 Live Sources ----- */
  .settings__input--sender {
    min-width: 0;
    width: 220px;
    font-family: var(--font-family-mono);
  }

  .settings__times {
    color: var(--color-text-3);
    font-size: var(--font-size-body-sm);
  }

  /* ----- Output routing tile (W27 PR2) ----- */

  .settings__output-routing {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    margin: var(--space-md) 0;
  }

  /* ----- Pill toggle (V-Sync, Custom resolution) ----- */

  .settings__pill-toggle {
    position: relative;
    width: 32px;
    height: 18px;
    border-radius: var(--radius-full);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    cursor: pointer;
    padding: 0;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.5);
    transition:
      background 0.15s ease,
      border-color 0.15s ease;
  }

  .settings__pill-toggle:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
  }

  .settings__pill-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--color-text-0);
    border: 1px solid var(--color-line-3);
    transition:
      transform 0.15s ease,
      background 0.15s ease,
      border-color 0.15s ease;
  }

  .settings__pill-toggle.on {
    background: var(--color-primary);
    border-color: var(--color-primary);
  }

  .settings__pill-toggle.on .settings__pill-thumb {
    transform: translateX(14px);
    background: var(--color-on-primary);
    border-color: var(--color-on-primary);
  }

  /* ----- Storage groups + folder rows ----- */

  .settings__storage-group {
    margin-bottom: var(--space-lg);
  }

  .settings__storage-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    margin-bottom: var(--space-sm);
  }

  .settings__storage-title {
    margin: 0;
    font-size: var(--font-size-display-sm);
    font-weight: 500;
    color: var(--color-text-1);
    text-transform: lowercase;
  }

  .settings__folder-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .settings__folder-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
  }

  .settings__folder-icon {
    color: var(--color-text-3);
    display: inline-flex;
    align-items: center;
  }

  .settings__folder-path {
    flex: 1;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
    word-break: break-all;
    min-width: 0;
  }

  .settings__folder-path--inline {
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-sm);
  }

  /* ----- Buttons ----- */

  .settings__btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-sans);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
    color: var(--color-text-1);
    transition:
      background 0.1s ease,
      border-color 0.1s ease,
      color 0.1s ease;
  }

  .settings__btn:hover {
    background: var(--color-bg-3);
    color: var(--color-text-0);
  }

  .settings__btn:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 1px;
  }

  .settings__btn--ghost {
    background: transparent;
  }

  .settings__btn--ghost:hover {
    background: var(--color-bg-2);
  }

  .settings__btn--danger {
    color: var(--color-danger);
    border-color: var(--color-line-2);
  }

  .settings__btn--danger:hover {
    background: var(--color-bg-3);
    border-color: var(--color-danger);
  }

  /* ----- Hotkeys table (preserved from W14) ----- */

  .settings__hotkey-group {
    margin-bottom: var(--space-md);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
    background: var(--color-bg-1);
  }

  .settings__hotkey-group > summary {
    padding: var(--space-sm) var(--space-md);
    font-weight: 600;
    cursor: pointer;
    user-select: none;
    color: var(--color-text-0);
    list-style: revert;
  }

  .settings__hotkey-group[open] > summary {
    border-bottom: 1px solid var(--color-line-1);
  }

  .settings__hotkey-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body-sm);
    color: var(--color-text-1);
  }

  .settings__hotkey-table th,
  .settings__hotkey-table td {
    padding: var(--space-xs) var(--space-md);
    text-align: left;
    border-bottom: 1px solid var(--color-line-1);
  }

  .settings__hotkey-table thead th {
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    background: var(--color-bg-2);
  }

  .settings__hotkey-table tbody tr:nth-child(even) {
    background: var(--color-bg-2);
  }

  .settings__hotkey-keys {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    flex-wrap: wrap;
  }

  .settings__hotkey-keys kbd {
    padding: 2px var(--space-xs);
    background: var(--color-bg-3);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-0);
  }

  .settings__sep {
    color: var(--color-text-3);
    margin: 0 2px;
  }

  .settings__hotkey-table code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-1);
  }

  .settings__cap {
    display: inline-block;
    padding: 1px var(--space-xs);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-xs);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-2);
    background: var(--color-bg-2);
  }

  .settings__muted {
    color: var(--color-text-3);
  }

  .settings__section button[disabled] {
    cursor: not-allowed;
    opacity: 0.6;
    background: var(--color-bg-2);
    color: var(--color-text-2);
    border: 1px solid var(--color-line-2);
    padding: var(--space-xs) var(--space-md);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body-sm);
  }

  .settings__about {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-xs) var(--space-md);
    margin: 0;
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
  }

  .settings__about dt {
    color: var(--color-text-2);
    text-transform: uppercase;
    font-size: var(--font-size-body-xs);
    letter-spacing: 0.05em;
    align-self: center;
  }

  .settings__about dd {
    margin: 0;
  }

  .settings__about code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    color: var(--color-text-0);
  }

  /* ----- Tempo & Sync (W22 PR1) ----- */

  .settings__bpm-chain {
    list-style: none;
    margin: 0 0 var(--space-md) 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .settings__bpm-row {
    display: grid;
    grid-template-columns: 1.5rem 0.6rem auto 1fr auto;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) var(--space-sm);
    background: var(--color-bg-1);
    border: 1px solid var(--color-line-1);
    border-radius: var(--radius-sm);
  }

  .settings__bpm-row.active {
    border-color: var(--color-primary);
    background: var(--color-bg-2);
  }

  .settings__bpm-rank {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-3);
    text-align: right;
  }

  .settings__bpm-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--color-text-3);
  }

  .settings__bpm-dot[data-state='active'] {
    background: var(--color-primary);
    box-shadow: 0 0 0 2px color-mix(in oklab, var(--color-primary) 25%, transparent);
  }

  .settings__bpm-name {
    color: var(--color-text-0);
    font-size: var(--font-size-body-sm);
  }

  .settings__bpm-row.active .settings__bpm-name {
    color: var(--color-primary);
  }

  .settings__bpm-detail {
    color: var(--color-text-2);
    font-size: var(--font-size-body-xs);
  }

  .settings__bpm-badge {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-on-primary);
    background: var(--color-primary);
    padding: 1px var(--space-xs);
    border-radius: var(--radius-xs);
  }

  .settings__bpm-readout {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-md);
    color: var(--color-text-0);
    font-variant-numeric: tabular-nums;
  }

  .settings__bpm-unit {
    color: var(--color-text-3);
    font-size: var(--font-size-body-xs);
    margin-left: 2px;
  }

  /* ----- W40 Ableton Link peer pill ----- */
  .settings__link-pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 2px var(--space-sm);
    background: var(--color-bg-3);
    color: var(--color-text-1);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    border-radius: var(--radius-full);
    border: 1px solid var(--color-line-2);
  }
  .settings__link-pill[data-state='active'] {
    color: var(--color-signal-cyan);
    border-color: color-mix(in oklab, var(--color-signal-cyan) 60%, transparent);
  }
  .settings__link-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-text-3);
  }
  .settings__link-pill[data-state='active'] .settings__link-dot {
    background: var(--color-signal-cyan);
    box-shadow: 0 0 0 2px color-mix(in oklab, var(--color-signal-cyan) 25%, transparent);
  }

  .settings__bpm-sensitivity {
    display: flex;
    align-items: flex-start;
    gap: var(--space-lg);
    margin: var(--space-md) 0;
  }

  .settings__bpm-sensitivity-meta {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    flex: 1;
    min-width: 0;
  }

  /* ----- W38 right rail: Preview tile + Diagnostics panel ----------- */

  .settings__rail {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    position: sticky;
    top: var(--space-xl);
    align-self: start;
    min-width: 0;
  }
  @media (max-width: 960px) {
    .settings__rail {
      position: static;
    }
  }
  .settings__preview {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .settings__preview-frame {
    aspect-ratio: 16 / 9;
    width: 100%;
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--color-bg-2);
    box-shadow: var(--shadow-panel);
  }
  .settings__preview-label {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-3);
    padding-left: var(--space-xs);
  }

  .settings__section--diag {
    /* override the main-flow spacing so the panel hugs the preview */
    margin-top: 0;
  }
  .settings__diag-data {
    margin: 0 0 var(--space-md) 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .settings__diag-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--space-md);
    padding: var(--space-xs) 0;
    border-bottom: 1px solid var(--color-line-1);
  }
  .settings__diag-row:last-of-type {
    border-bottom: none;
  }
  .settings__diag-key {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-caps-sm);
    text-transform: uppercase;
    letter-spacing: var(--letter-spacing-caps-sm);
    color: var(--color-text-2);
    margin: 0;
  }
  .settings__diag-value {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-mono-sm);
    color: var(--color-text-1);
    margin: 0;
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
</style>
