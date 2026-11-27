<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

<script lang="ts">
  // Wave-21 PR1 Settings expansion — extends the W14 PR3 Settings view
  // (which shipped Hotkeys + ThemeSwitcher only) with the four operator
  // bring-up sections flagged by the W20 feature-parity report
  // (`/tmp/feature-parity-report.md`):
  //
  //   1. Audio Device — input/output picker, sample rate, latency hint,
  //      channel layout. Input device enumeration live-wired per Wave 2.6
  //      AUD.A; output device + sample rate / latency / channel layout /
  //      channel mapping persist via TOML but real engine apply is
  //      deferred.
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
  // The form fields below read and write the persisted settings store.
  // Runtime application of a few preferences is still subsystem-owned,
  // but the operator document itself survives restart.
  import { onDestroy } from 'svelte';

  import {
    ampToDb,
    ExpEnvelope,
    PeakBallistics,
    PeakHold,
    SilenceGate,
    VU_INTEGRATION_TAU_MS,
  } from '../audio/metering';
  import { HOTKEYS } from '../data/hotkeys';
  import {
    ABLETON_LINK_PEER_FIXTURE,
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
    type ColorSpaceOption,
    type FolderEntry,
    type MultiDisplayModeOption,
    type QuantizeTransitionOption,
  } from '../data/settings-fixtures';
  import Icon from '../Icon.svelte';
  import { pickFolder } from '../ipc/picker';
  import {
    getAudioChannelPair,
    getDjDeviceProfile,
    inferDjDeviceProfileFallback,
    setAudioChannelPair,
    type DjDeviceProfilePayload,
  } from '../ipc/showtime';
  import { audioDevicesStore } from '../stores/audioDevices.svelte';
  import { audioSnapshotStore } from '../stores/audioSnapshot.svelte';
  import { settingsStore } from '../stores/settings.svelte';
  import ThemeSwitcher from '../ThemeSwitcher.svelte';
  import { ControlGroup, Fader, Meter, OutputRoutingTile, TabSegment } from '../widgets';
  import SceneStill from '../widgets/SceneStill.svelte';

  // Wave 2.1 Lane C.3 — persist Audio / Graphics / General settings
  // via the `get_settings` / `set_settings` Tauri IPC handlers.
  // Storage / Live Sources stay local-state-only at this slice; they
  // round-trip through the bundle-faithful fixture defaults and
  // don't survive restart yet. Wave 2.7 Lane SET.A widens persistence
  // to cover Tempo & Sync (Ableton Link / Start-Stop Sync / MIDI
  // Clock In + Out / Quantize / Manual BPM / Link offset / Audio BPM
  // sensitivity / BPM source priority) via the new
  // `settingsStore.update({ tempo: ... })` patch shape and also
  // unifies Theme persistence: the ThemeSwitcher now reads / writes
  // `settingsStore.snapshot.general.theme` (localStorage retained as
  // a fast-path fallback for the no-Tauri / pre-load window).
  //
  // The store loads asynchronously on mount; until it resolves the
  // form fields render against the bundle-faithful defaults baked
  // into `settingsStore.snapshot` (which match the fixture defaults
  // verbatim, so the existing Settings view tests keep passing).
  $effect(() => {
    void settingsStore.load();
  });

  // Wave 2.6 Lane AUD.A — populate the Input Device dropdown with the
  // host's real cpal input devices via `list_audio_devices`. Empty
  // list (CI / headless / no-audio-subsystem hosts; running outside
  // Tauri under `npm run dev` / Vitest) falls back to the W21 PR1
  // `AUDIO_INPUT_DEVICES` fixture so the dropdown always renders at
  // least the default-microphone hint. Mirrors the Wave 2.1 Lane D.1
  // MIDI Devices tab refresh-on-mount pattern.
  $effect(() => {
    void audioDevicesStore.refresh();
  });

  // Wave 2.7 Lane SET.C — Output Device dropdown mirror of the AUD.A
  // input enumeration. Same fixture-fallback posture under the no-
  // audio-subsystem / no-Tauri-context branches; empty live list
  // collapses to `AUDIO_OUTPUT_DEVICES`.
  $effect(() => {
    void audioDevicesStore.refreshOutput();
  });

  // Pull the audio + graphics + general sections through derived
  // reads so any store mutation reflects in the UI on next $effect
  // flush. Wave 2.7 SET.B extends the derived set to cover storage
  // + live_sources so those sections now persist through the
  // settingsStore IPC seam as well.
  const audioSnapshot = $derived(settingsStore.snapshot.audio);
  const graphicsSnapshot = $derived(settingsStore.snapshot.graphics);
  const storageSnapshot = $derived(settingsStore.snapshot.storage);
  const liveSourcesSnapshot = $derived(settingsStore.snapshot.live_sources);

  // Wave 2.6 AUD.B — live audio monitoring meter wiring.
  // Subscribe to the audio-uniform IPC store on mount so the
  // Monitoring meter in the Audio Device section reads live
  // `rms` / `peak` from the engine producer (Wave 1c-α-2). The
  // subscription is idempotent across widgets; calling it here
  // simply joins the in-flight latch the visualizer widgets
  // already share. When no snapshot has arrived (running outside
  // Tauri / CI / device-less hosts), the meter falls back to the
  // bundle-faithful frozen-frame fixtures per the dual-mode posture
  // established by `AudioReactivity.svelte`.
  $effect(() => {
    void audioSnapshotStore.subscribe();
  });

  // World-class metering — drive the Monitoring meter through the shared
  // dt-aware ballistics primitive (`src/lib/audio/metering.ts`) instead
  // of an ad-hoc per-frame EMA. Level uses a VU-style RMS integration;
  // the held peak uses PPM fast-attack + linear release into a peak-hold
  // (hold-time + dB/s decay); a silence gate drives the no-signal state.
  // All operate in the dBFS domain on the real wall-clock dt between
  // engine snapshots, so the meter is calibrated and frame-rate
  // independent. When no snapshot has arrived (outside Tauri / CI /
  // device-less hosts) the meter shows the bundle-faithful frozen-frame
  // fixture levels and the ballistics reset so the next live snapshot
  // seeds cleanly.
  const levelEnv = new ExpEnvelope(VU_INTEGRATION_TAU_MS);
  const peakBallistics = new PeakBallistics();
  const peakHold = new PeakHold();
  const silenceGate = new SilenceGate();

  const FIXTURE_LEVEL_DB = ampToDb(AUDIO_METER_LEVEL);
  const FIXTURE_PEAK_DB = ampToDb(AUDIO_METER_PEAK);

  let meterLevelDb = $state<number>(FIXTURE_LEVEL_DB);
  let meterPeakDb = $state<number>(FIXTURE_PEAK_DB);
  let meterSilent = $state<boolean>(false);
  let _lastSnapMs = 0;
  let _meterSeeded = false;

  $effect(() => {
    const snap = audioSnapshotStore.snapshot;
    if (!snap) {
      meterLevelDb = FIXTURE_LEVEL_DB;
      meterPeakDb = FIXTURE_PEAK_DB;
      meterSilent = false;
      _meterSeeded = false;
      levelEnv.reset();
      peakBallistics.reset();
      peakHold.reset();
      silenceGate.reset();
      return;
    }
    const now = typeof performance !== 'undefined' ? performance.now() : 0;
    // First live snapshot seeds instantly (large dt saturates the
    // envelopes onto the target); subsequent steps use the real dt,
    // capped so a backgrounded tab doesn't fast-forward the decay.
    const dtMs = _meterSeeded ? Math.min(250, Math.max(0, now - _lastSnapMs)) : 1e4;
    _lastSnapMs = now;
    _meterSeeded = true;

    const levelDb = levelEnv.process(ampToDb(snap.rms), dtMs);
    const fastPeakDb = peakBallistics.process(ampToDb(snap.peak), dtMs);
    meterLevelDb = levelDb;
    meterPeakDb = peakHold.process(fastPeakDb, dtMs);
    meterSilent = silenceGate.update(levelDb, dtMs);
  });

  // Always-present built-in capture source rendered ahead of the
  // enumerated devices. `system-audio` is the zero-install
  // system-output loopback capture (macOS 14.6+ Core Audio taps /
  // Windows WASAPI loopback) that engine/audio's
  // `try_input_by_preference` routes to `try_system_audio_loopback`.
  // Exposing it as a first-class entry (parallel to the reference's
  // "Desktop Audio") means reacting to whatever the machine is PLAYING
  // needs no hand-installed BlackHole/Loopback driver. The host default
  // input (`default`) is already provided by the enumerated / fixture
  // list, so only `system-audio` is prepended here.
  const FIXED_AUDIO_INPUT_OPTIONS = [{ id: 'system-audio', label: 'System Audio (built-in)' }];

  // Wave 2.6 Lane AUD.A — derive the Input Device dropdown option set
  // from `audioDevicesStore.devices`. Empty live list falls back to
  // the W21 PR1 fixture so the dropdown always has something to
  // render (e.g. running outside Tauri). The fixed system sources are
  // always prepended so `system-audio` / `default` are selectable
  // regardless of the enumerated-device state.
  const liveAudioInputOptions = $derived(
    audioDevicesStore.devices.map((d) => ({ id: d.id, label: d.name })),
  );
  const audioInputOptions = $derived([
    ...FIXED_AUDIO_INPUT_OPTIONS,
    ...(liveAudioInputOptions.length > 0 ? liveAudioInputOptions : AUDIO_INPUT_DEVICES),
  ]);

  // Wave 2.7 Lane SET.C — same dual-mode pattern for Output Device.
  const liveAudioOutputOptions = $derived(
    audioDevicesStore.outputDevices.map((d) => ({ id: d.id, label: d.name })),
  );
  const audioOutputOptions = $derived(
    liveAudioOutputOptions.length > 0 ? liveAudioOutputOptions : AUDIO_OUTPUT_DEVICES,
  );

  // Right-rail Diagnostics rows. Show honest idle/off defaults when no
  // live runtime adapter snapshot is active rather than hardcoded fake
  // telemetry values.
  interface DiagnosticsRow {
    readonly key: string;
    readonly value: string;
  }
  const diagnosticsRows = $derived<readonly DiagnosticsRow[]>([
    { key: 'GPU', value: 'Idle' },
    { key: 'FPS / Frame', value: 'Idle' },
    {
      key: 'Audio Latency',
      value: audioSnapshotStore.snapshot ? 'Live' : 'Idle',
    },
    { key: 'MIDI Devices', value: '0 connected' },
    {
      key: 'Syphon / Spout',
      value:
        liveSourcesSnapshot.syphon_output_enabled || liveSourcesSnapshot.syphon_input_enabled
          ? 'Idle'
          : 'Off',
    },
    {
      key: 'Ableton Link',
      value: settingsStore.snapshot.tempo.ableton_link_enabled ? 'Idle · 0 peers' : 'Off',
    },
  ]);

  /* ----- Audio Device — persisted via settingsStore -----
   * The form fields below read live from `settingsStore.snapshot.audio`
   * (via the `audioSnapshot` $derived above) and write back through
   * `settingsStore.update({ audio: { ... } })`. The store debounces
   * disk persistence so rapid slider drags collapse into one save. */
  const inputGainDb = $derived((audioSnapshot.input_gain - 0.5) * AUDIO_INPUT_GAIN_DB_RANGE_DB);
  const inputGainDbLabel = $derived(`${inputGainDb >= 0 ? '+' : ''}${inputGainDb.toFixed(1)} dB`);

  /* ----- DJ / Rekordbox Master vs. Cue Routing -----
   * Queries `get_dj_device_profile` whenever the selected input device
   * changes (with a pure client-side fallback heuristic outside Tauri)
   * and binds the active stereo channel pair to `set_audio_channel_pair`
   * so Headphone Cue (Ch 3-4) stays isolated from visuals. */
  interface AudioChannelPairOption {
    readonly id: string;
    readonly left: number;
    readonly right: number;
    readonly label: string;
  }

  const DJ_CHANNEL_PAIR_OPTIONS: readonly AudioChannelPairOption[] = [
    { id: '0-1', left: 0, right: 1, label: 'Ch 1–2 (Master / REC OUT)' },
    { id: '2-3', left: 2, right: 3, label: 'Ch 3–4 (Headphone Cue / Deck)' },
    { id: '4-5', left: 4, right: 5, label: 'Ch 5–6' },
    { id: '6-7', left: 6, right: 7, label: 'Ch 7–8' },
    { id: '8-9', left: 8, right: 9, label: 'Ch 9–10' },
  ];

  const selectedInputDeviceName = $derived(
    audioInputOptions.find((opt) => opt.id === audioSnapshot.input_device)?.label ??
      audioSnapshot.input_device,
  );

  let djProfile = $state<DjDeviceProfilePayload>(inferDjDeviceProfileFallback('default', 4));
  let selectedChannelPairId = $state<string>('0-1');

  $effect(() => {
    const deviceName = selectedInputDeviceName;
    djProfile = inferDjDeviceProfileFallback(deviceName, 4);
    let cancelled = false;
    void getDjDeviceProfile(deviceName, 4).then((resolved) => {
      if (!cancelled && resolved) {
        djProfile = resolved;
      }
    });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    let cancelled = false;
    void getAudioChannelPair().then((pair) => {
      if (!cancelled && pair) {
        const key = `${pair.left_channel}-${pair.right_channel}`;
        if (DJ_CHANNEL_PAIR_OPTIONS.some((opt) => opt.id === key)) {
          selectedChannelPairId = key;
        }
      }
    });
    return () => {
      cancelled = true;
    };
  });

  const djProfileBadgeLabel = $derived(
    djProfile.recognized
      ? djProfile.kind === 'ddj_flx10'
        ? 'DDJ-FLX10 Detected'
        : 'Rekordbox / DJ Profile'
      : 'Stereo / Multi-Channel',
  );

  const isCuePairWarningActive = $derived.by(() => {
    if (!djProfile.recognized) return false;
    const cueKey = djProfile.cue_pair
      ? `${djProfile.cue_pair.left_channel}-${djProfile.cue_pair.right_channel}`
      : '2-3';
    return selectedChannelPairId === cueKey;
  });

  function handleChannelPairChange(pairId: string): void {
    selectedChannelPairId = pairId;
    const option = DJ_CHANNEL_PAIR_OPTIONS.find((opt) => opt.id === pairId);
    if (option) {
      void setAudioChannelPair(option.left, option.right);
    }
  }

  /* ----- Graphics / Output — persisted via settingsStore -----
   * Same pattern as Audio: read through `graphicsSnapshot` (the
   * $derived above); write through `settingsStore.update({ graphics })`.
   * W21 PR1 + W41 cohort fields all flow through the same seam. */
  const renderScaleLabel = $derived(`${graphicsSnapshot.render_scale.toFixed(2)}×`);

  /* ----- Live Sources (Wave 2.7 SET.B) -----
   * Bundle source: views/settings.jsx lines 115-124. Six bundle-
   * faithful rows (five boolean toggles + one text input). The
   * engine-side capability gating for Spout / Syphon / NDI is a
   * Wave 1c-β follow-up; the UI prefs persist NOW through the
   * settingsStore TOML round-trip so operators don't keep re-
   * flipping the same six prefs across restarts. `data-cap`
   * attributes (`io.syphon.*`, `io.ndi.*`, `render.custom-resolution`)
   * ride along for the W15+ entitlement infra. */

  /* ----- Storage (Wave 2.7 SET.B) -----
   * Storage section's four persisted axes (media_folders +
   * scene_folders + backup_path + backup_retention) now read from
   * `settingsStore.snapshot.storage` and write through
   * `settingsStore.update({ storage })`. The bundle-faithful fixture
   * defaults (`DEFAULT_MEDIA_FOLDERS` / `DEFAULT_SCENE_FOLDERS` /
   * `DEFAULT_BACKUP_PATH`) seed the store on first launch via the
   * one-time `$effect` below; once seeded, every Add / Remove /
   * Browse action mutates the store directly and the in-memory
   * snapshot reflects the change on next render. */

  /* `FolderEntry` carries a synthetic `id` for the `{#each}` key,
   * synthesised from the path (which is unique per row since the
   * Storage section disallows duplicates by construction). */
  const mediaFolders: FolderEntry[] = $derived(
    storageSnapshot.media_folders.map((path) => ({ id: `mf-${path}`, path })),
  );
  const sceneFolders: FolderEntry[] = $derived(
    storageSnapshot.scene_folders.map((path) => ({ id: `sf-${path}`, path })),
  );
  const backupPath = $derived(
    storageSnapshot.backup_path !== '' ? storageSnapshot.backup_path : DEFAULT_BACKUP_PATH,
  );

  /* Browser-mode fixture hydration. The shell load settles before any
   * seed can run, and persisted Tauri settings are never clobbered.
   * When `get_settings` is unavailable (Vite / Vitest), seed the
   * local in-memory snapshot so the storage rows still render with the
   * bundle's starter folders; `settingsStore.canPersist` remains false
   * in that branch, so no write-through is scheduled. */
  let storageSeedAttempted = $state(false);
  $effect(() => {
    if (!settingsStore.isLoaded || settingsStore.canPersist || storageSeedAttempted) {
      return;
    }
    storageSeedAttempted = true;
    const s = settingsStore.snapshot.storage;
    const isEmpty =
      s.media_folders.length === 0 && s.scene_folders.length === 0 && s.backup_path === '';
    if (isEmpty) {
      settingsStore.update({
        storage: {
          media_folders: DEFAULT_MEDIA_FOLDERS.map((f) => f.path),
          scene_folders: DEFAULT_SCENE_FOLDERS.map((f) => f.path),
          backup_path: DEFAULT_BACKUP_PATH,
        },
      });
    }
  });

  async function browseMediaFolder(replacePath?: string) {
    const path = await pickFolder('Pick a Media folder');
    if (!path) return;
    const current = settingsStore.snapshot.storage.media_folders;
    const next =
      replacePath !== undefined
        ? current.map((p) => (p === replacePath ? path : p))
        : [...current, path];
    const deduped = Array.from(new Set(next));
    settingsStore.update({ storage: { media_folders: deduped } });
  }

  async function browseSceneFolder(replacePath?: string) {
    const path = await pickFolder('Pick a Custom scene folder');
    if (!path) return;
    const current = settingsStore.snapshot.storage.scene_folders;
    const next =
      replacePath !== undefined
        ? current.map((p) => (p === replacePath ? path : p))
        : [...current, path];
    const deduped = Array.from(new Set(next));
    settingsStore.update({ storage: { scene_folders: deduped } });
  }

  async function browseBackupFolder() {
    const path = await pickFolder('Pick a Backups folder');
    if (path) {
      settingsStore.update({ storage: { backup_path: path } });
    }
  }

  function removeMediaFolder(id: string) {
    // The synthetic `id` is `mf-<path>` per the derived mapping
    // above; strip the prefix to recover the path and filter.
    const path = id.startsWith('mf-') ? id.slice(3) : id;
    settingsStore.update({
      storage: {
        media_folders: settingsStore.snapshot.storage.media_folders.filter((p) => p !== path),
      },
    });
  }

  function removeSceneFolder(id: string) {
    const path = id.startsWith('sf-') ? id.slice(3) : id;
    settingsStore.update({
      storage: {
        scene_folders: settingsStore.snapshot.storage.scene_folders.filter((p) => p !== path),
      },
    });
  }

  /* ----- Tempo & Sync — persisted via settingsStore (Wave 2.7 SET.A) -----
   * Replaces the W22 PR1 / W40 local `$state` set with reads through
   * `settingsStore.snapshot.tempo` and writes through
   * `settingsStore.update({ tempo: { ... } })`. Runtime wiring (Audio
   * estimator, MIDI Clock, Ableton Link peer discovery) still lands
   * in Phase-3+; Wave 2.7 closes only the persistence seam so
   * operator-tuned tempo values survive restart.
   *
   * The priority chain is FIXED at the default W22 order today and
   * persisted as a string array on the engine. Drag-reorder + per-
   * source health remain Phase-3+ scope per the M-10 BPM source
   * resolver in
   * `chromaticity-planning/ui-predev-bundle-v0.2-beta/views/midi.jsx`. */

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

  const tempoSnapshot = $derived(settingsStore.snapshot.tempo);

  // Tap-tempo state stays local — transient UI computation, not
  // operator-tunable preference. The computed BPM, when the
  // operator clicks "Set", round-trips through settingsStore to
  // become the new persisted manual_bpm.
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

  onDestroy(() => {
    if (tapResetTimer !== null) {
      clearTimeout(tapResetTimer);
      tapResetTimer = null;
    }
  });

  function setManualBpm() {
    // Prefer active tap-tempo reading if present; clamp into canonical 20..=300.
    const candidate = tappedBpm ?? tempoSnapshot.manual_bpm;
    const raw = Number.isFinite(candidate) ? candidate : 120;
    const clamped = Math.min(300, Math.max(20, Math.round(raw * 10) / 10));
    settingsStore.update({ tempo: { manual_bpm: clamped } });
  }

  /* ----- Wave 2.7 Lane SET.D: Reset-to-Defaults + live About version -----
   * Reset-to-Defaults surfaces the `reset_settings` Tauri command
   * through a Maintenance section between Hotkeys and About. A
   * confirmation modal guards the destructive action; on confirm,
   * `settingsStore.resetToDefaults()` swaps the in-memory document
   * AND rewrites the on-disk TOML file with `Settings::default()`.
   *
   * About version reads `getVersion()` / `getTauriVersion()` from
   * `@tauri-apps/api/app` at runtime so the displayed version stays
   * in sync with `Cargo.toml` / the Tauri runtime — no more hardcoded
   * `v0.0.0`. Repo + License + Phase lines stay literal (governance
   * constants, not build artefacts). Falls back to "n/a" when the
   * `@tauri-apps/api/app` import fails (Vitest jsdom; vite-only dev
   * server). */
  let resetConfirmOpen = $state(false);
  let resetInFlight = $state(false);
  let appVersion = $state<string>('n/a');
  let tauriVersion = $state<string>('n/a');

  $effect(() => {
    void (async () => {
      try {
        const { getVersion, getTauriVersion } = await import('@tauri-apps/api/app');
        const [v, t] = await Promise.all([getVersion(), getTauriVersion()]);
        appVersion = v;
        tauriVersion = t;
      } catch {
        // Outside Tauri runtime (Vitest jsdom, vite-only dev). The
        // About section displays the "n/a" fallback so the view
        // still renders cleanly.
      }
    })();
  });

  function openResetConfirm() {
    resetConfirmOpen = true;
  }

  function closeResetConfirm() {
    if (!resetInFlight) {
      resetConfirmOpen = false;
    }
  }

  async function confirmReset() {
    if (resetInFlight) return;
    resetInFlight = true;
    try {
      await settingsStore.resetToDefaults();
    } finally {
      resetInFlight = false;
      resetConfirmOpen = false;
    }
  }
</script>

<section class="settings">
  <div class="settings__main">
    <header class="settings__header">
      <h1>settings</h1>
      <p class="settings__intro">
        Operator preferences, devices, output, storage, tempo sync, hotkey reference, and
        diagnostics. Changes are saved automatically.
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
            {#each audioInputOptions as opt (opt.id)}
              <option value={opt.id}>{opt.label}</option>
            {/each}
          </select>
          <p class="settings__hint settings__hint--field">
            Pick <strong>System Audio (built-in)</strong> to react to whatever your computer is
            playing — no extra software to install (macOS 14.6+ / Windows; macOS asks once for
            audio-recording permission). On older macOS, or to capture one specific app, install
            <a href="https://existential.audio/blackhole/" target="_blank" rel="noopener noreferrer"
              >BlackHole</a
            >
            (free, MIT) or
            <a href="https://rogueamoeba.com/loopback/" target="_blank" rel="noopener noreferrer"
              >Loopback</a
            >
            and select it here. Restart to apply device changes.
          </p>
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
            {#each audioOutputOptions as opt (opt.id)}
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
        <div class="settings__field" data-testid="dj-routing-row">
          <span class="settings__label">DJ / Master Bus</span>
          <div class="settings__field-row">
            <label class="settings__a11y-only" for="dj-channel-pair">Channel Pair</label>
            <select
              id="dj-channel-pair"
              class="settings__select"
              value={selectedChannelPairId}
              onchange={(e) => handleChannelPairChange(e.currentTarget.value)}
            >
              {#each DJ_CHANNEL_PAIR_OPTIONS as opt (opt.id)}
                <option value={opt.id}>{opt.label}</option>
              {/each}
            </select>
            <span
              class="settings__dj-badge"
              class:settings__dj-badge--recognized={djProfile.recognized}
              data-testid="dj-profile-badge"
            >
              {djProfileBadgeLabel}
            </span>
          </div>
          <p class="settings__hint settings__hint--field" data-testid="dj-profile-note">
            {djProfile.notes}
          </p>
          {#if isCuePairWarningActive}
            <p
              class="settings__dj-warn settings__hint--field"
              role="status"
              data-testid="dj-cue-warning"
            >
              Cue pair selected — Headphone Cue may leak into visuals. Use Ch 1–2 (Master).
            </p>
          {/if}
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
              levelDb={meterLevelDb}
              peakDb={meterPeakDb}
              silent={meterSilent}
              bars={AUDIO_METER_BARS}
              showReadout
              onResetPeak={() => {
                peakHold.reset();
                meterPeakDb = meterLevelDb;
              }}
              ariaLabel="Audio input monitor"
            />
          </div>
        </div>
      </div>
      <p class="settings__hint">
        Device choices are saved and applied on restart. The monitoring meter displays live input
        levels when an audio device is active.
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
              oninput={(e) => {
                const parsed = Number.parseInt(e.currentTarget.value, 10);
                if (Number.isFinite(parsed) && parsed >= 320 && parsed <= 7680) {
                  settingsStore.update({ graphics: { custom_width: parsed - (parsed % 2) } });
                }
              }}
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
              oninput={(e) => {
                const parsed = Number.parseInt(e.currentTarget.value, 10);
                if (Number.isFinite(parsed) && parsed >= 240 && parsed <= 4320) {
                  settingsStore.update({ graphics: { custom_height: parsed - (parsed % 2) } });
                }
              }}
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
              output window controls coming soon
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
        Custom resolution and HDR output require a supported display and license tier.
      </p>

      <div class="settings__output-routing">
        <span class="settings__label">Active output routing</span>
        <OutputRoutingTile />
      </div>
      <p class="settings__hint">
        Displays active video output destinations including connected displays, Syphon/Spout, and
        NDI streams.
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
            class:on={liveSourcesSnapshot.syphon_input_enabled}
            aria-pressed={liveSourcesSnapshot.syphon_input_enabled}
            aria-label="Spout / Syphon Input"
            onclick={() =>
              settingsStore.update({
                live_sources: {
                  syphon_input_enabled: !liveSourcesSnapshot.syphon_input_enabled,
                },
              })}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field" data-cap="io.syphon.output">
          <span class="settings__label">Spout / Syphon Output</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={liveSourcesSnapshot.syphon_output_enabled}
            aria-pressed={liveSourcesSnapshot.syphon_output_enabled}
            aria-label="Spout / Syphon Output"
            onclick={() =>
              settingsStore.update({
                live_sources: {
                  syphon_output_enabled: !liveSourcesSnapshot.syphon_output_enabled,
                },
              })}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field" data-cap="io.ndi.input">
          <span class="settings__label">NDI Input</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={liveSourcesSnapshot.ndi_input_enabled}
            aria-pressed={liveSourcesSnapshot.ndi_input_enabled}
            aria-label="NDI Input"
            onclick={() =>
              settingsStore.update({
                live_sources: { ndi_input_enabled: !liveSourcesSnapshot.ndi_input_enabled },
              })}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field" data-cap="io.ndi.output">
          <span class="settings__label">NDI Output</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={liveSourcesSnapshot.ndi_output_enabled}
            aria-pressed={liveSourcesSnapshot.ndi_output_enabled}
            aria-label="NDI Output"
            onclick={() =>
              settingsStore.update({
                live_sources: { ndi_output_enabled: !liveSourcesSnapshot.ndi_output_enabled },
              })}
          >
            <span class="settings__pill-thumb"></span>
          </button>
        </div>
        <div class="settings__field" data-cap="render.custom-resolution">
          <span class="settings__label">Constant Output Resolution</span>
          <button
            type="button"
            class="settings__pill-toggle"
            class:on={liveSourcesSnapshot.constant_output_resolution}
            aria-pressed={liveSourcesSnapshot.constant_output_resolution}
            aria-label="Constant Output Resolution"
            onclick={() =>
              settingsStore.update({
                live_sources: {
                  constant_output_resolution: !liveSourcesSnapshot.constant_output_resolution,
                },
              })}
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
            value={liveSourcesSnapshot.sender_name}
            oninput={(e) =>
              settingsStore.update({
                live_sources: { sender_name: e.currentTarget.value },
              })}
            maxlength="32"
          />
        </div>
      </div>
      <p class="settings__hint">
        Live texture input and output preferences are saved automatically. Enable Spout, Syphon, or
        NDI to share GPU textures with other applications.
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
            onclick={() => void browseMediaFolder()}
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
                  onclick={() => void browseMediaFolder(folder.path)}
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
            onclick={() => void browseSceneFolder()}
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
                  onclick={() => void browseSceneFolder(folder.path)}
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
            <select
              id="backup-retention"
              class="settings__select"
              value={storageSnapshot.backup_retention}
              onchange={(e) =>
                settingsStore.update({
                  storage: { backup_retention: e.currentTarget.value },
                })}
            >
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
          <button
            type="button"
            class="settings__btn settings__btn--ghost"
            disabled
            title="Thumbnail cache cleanup is coming in a future release"
          >
            <Icon name="refresh" size={12} /> Clear thumbnails
          </button>
          <button
            type="button"
            class="settings__btn settings__btn--ghost"
            disabled
            title="Shader render cache cleanup is coming in a future release"
          >
            <Icon name="refresh" size={12} /> Clear render cache
          </button>
        </div>
        <p class="settings__hint">
          Cache cleanup clears stored scene thumbnails and compiled shader caches.
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
          First active tempo source wins. Automatic fallback follows the priority order below.
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
              value={tempoSnapshot.manual_bpm}
              oninput={(e) => {
                const parsed = Number.parseFloat(e.currentTarget.value);
                if (Number.isFinite(parsed) && parsed >= 20 && parsed <= 300) {
                  settingsStore.update({ tempo: { manual_bpm: parsed } });
                }
              }}
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
              value={tempoSnapshot.link_offset_ms}
              oninput={(e) => {
                const parsed = Number.parseFloat(e.currentTarget.value);
                if (Number.isFinite(parsed) && parsed >= -200 && parsed <= 200) {
                  settingsStore.update({ tempo: { link_offset_ms: parsed } });
                }
              }}
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
              class:on={tempoSnapshot.ableton_link_enabled}
              aria-pressed={tempoSnapshot.ableton_link_enabled}
              aria-label="Ableton Link"
              onclick={() =>
                settingsStore.update({
                  tempo: { ableton_link_enabled: !tempoSnapshot.ableton_link_enabled },
                })}
            >
              <span class="settings__pill-thumb"></span>
            </button>
            <span
              class="settings__link-pill"
              data-state={tempoSnapshot.ableton_link_enabled ? 'active' : 'idle'}
            >
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
              class:on={tempoSnapshot.start_stop_sync}
              aria-pressed={tempoSnapshot.start_stop_sync}
              aria-label="Start Stop Sync"
              onclick={() =>
                settingsStore.update({
                  tempo: { start_stop_sync: !tempoSnapshot.start_stop_sync },
                })}
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
              class:on={tempoSnapshot.midi_clock_in_enabled}
              aria-pressed={tempoSnapshot.midi_clock_in_enabled}
              aria-label="MIDI Clock Input"
              onclick={() =>
                settingsStore.update({
                  tempo: { midi_clock_in_enabled: !tempoSnapshot.midi_clock_in_enabled },
                })}
            >
              <span class="settings__pill-thumb"></span>
            </button>
            <select
              id="midi-clock-in-device"
              class="settings__select settings__select--compact"
              value={tempoSnapshot.midi_clock_in_device}
              onchange={(e) =>
                settingsStore.update({
                  tempo: { midi_clock_in_device: e.currentTarget.value },
                })}
              disabled={!tempoSnapshot.midi_clock_in_enabled}
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
              class:on={tempoSnapshot.midi_clock_out_enabled}
              aria-pressed={tempoSnapshot.midi_clock_out_enabled}
              aria-label="MIDI Clock Output"
              onclick={() =>
                settingsStore.update({
                  tempo: { midi_clock_out_enabled: !tempoSnapshot.midi_clock_out_enabled },
                })}
            >
              <span class="settings__pill-thumb"></span>
            </button>
            <select
              id="midi-clock-out-device"
              class="settings__select settings__select--compact"
              value={tempoSnapshot.midi_clock_out_device}
              onchange={(e) =>
                settingsStore.update({
                  tempo: { midi_clock_out_device: e.currentTarget.value },
                })}
              disabled={!tempoSnapshot.midi_clock_out_enabled}
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
              value={tempoSnapshot.quantize as QuantizeTransitionOption['id']}
              onChange={(id) => settingsStore.update({ tempo: { quantize: id } })}
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
          <Fader
            value={tempoSnapshot.audio_bpm_sensitivity / 100}
            label="BPM sense"
            onchange={(next) =>
              settingsStore.update({ tempo: { audio_bpm_sensitivity: next * 100 } })}
          />
        </ControlGroup>
        <div class="settings__bpm-sensitivity-meta">
          <span class="settings__label">Audio BPM sensitivity</span>
          <span class="settings__bpm-readout">
            {Math.round(tempoSnapshot.audio_bpm_sensitivity)}<span class="settings__bpm-unit"
              >%</span
            >
          </span>
          <p class="settings__hint">
            Tuning weight applied to the audio onset detector before it competes in the tempo
            priority chain.
          </p>
        </div>
      </div>

      <p class="settings__hint">
        Tempo and sync preferences are saved automatically and restored on launch.
      </p>
    </article>

    <article class="settings__section">
      <h2>General</h2>
      <div class="settings__row">
        <ThemeSwitcher />
        <p class="settings__hint">
          Switch between Pulse, Atelier, and Stage visual themes. Your selection is saved
          automatically.
        </p>
      </div>
      <div class="settings__row settings__row--locked" data-cap="io.osc.input">
        <label class="settings__toggle">
          <input type="checkbox" disabled checked title="Managed by your active license tier" />
          <span>OSC input adapter</span>
        </label>
        <p class="settings__hint">Managed by your active license tier.</p>
      </div>
      <div class="settings__row settings__row--locked" data-cap="io.camera">
        <label class="settings__toggle">
          <input type="checkbox" disabled title="Managed by your active license tier" />
          <span>Live Sources (camera capture)</span>
        </label>
        <p class="settings__hint">Managed by your active license tier.</p>
      </div>
    </article>

    <article class="settings__section">
      <h2>Hotkeys</h2>
      <p class="settings__hint">
        Read-only reference. Press <kbd>?</kbd> anywhere to open the quick hotkey overlay.
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
      <h2>Maintenance</h2>
      <p class="settings__hint">
        Reset every operator preference (Audio, Graphics, Storage, Tempo, OSC, and General) back to
        factory defaults. This overwrites your saved settings file.
      </p>
      <div class="settings__row">
        <button type="button" class="settings__danger" onclick={openResetConfirm}>
          Reset to Defaults
        </button>
      </div>
    </article>

    <article class="settings__section">
      <h2>About</h2>
      <dl class="settings__about">
        <dt>version</dt>
        <dd>v{appVersion}</dd>
        <dt>channel</dt>
        <dd>Pre-release</dd>
        <dt>license</dt>
        <dd><code>LicenseRef-Chromaticity-Proprietary</code></dd>
        <dt>repo</dt>
        <dd><code>PhantasticUniverse/chromaticity</code></dd>
        <dt>toolkit</dt>
        <dd>Tauri {tauriVersion} + Svelte 5 + Rust</dd>
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
        {#each diagnosticsRows as row (row.key)}
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
          title="Export logs, scene state, and adapter diagnostics as a support bundle"
        >
          <Icon name="save" size={12} /> Generate Support Bundle
        </button>
        <button type="button" disabled title="Open live runtime log viewer">View Logs</button>
      </div>
      <p class="settings__hint">
        System telemetry and adapter health metrics for live performance troubleshooting.
      </p>
    </article>
  </aside>
</section>

<svelte:window
  onkeydown={(e) => {
    if (resetConfirmOpen && e.key === 'Escape') {
      closeResetConfirm();
    }
  }}
/>

{#if resetConfirmOpen}
  <div
    class="settings__modal-backdrop"
    role="presentation"
    onclick={closeResetConfirm}
    onkeydown={(e) => {
      if (e.key === 'Escape') closeResetConfirm();
    }}
  ></div>
  <div
    class="settings__modal"
    role="dialog"
    aria-modal="true"
    aria-labelledby="reset-confirm-title"
    aria-describedby="reset-confirm-body"
  >
    <h2 id="reset-confirm-title" class="settings__modal-title">Reset settings to defaults?</h2>
    <p id="reset-confirm-body" class="settings__modal-body">
      This will reset all settings (Audio / Graphics / Storage / OSC / General) to the engine
      defaults and overwrite the persisted settings file. This cannot be undone.
    </p>
    <div class="settings__modal-actions">
      <button type="button" onclick={closeResetConfirm} disabled={resetInFlight}>Cancel</button>
      <button
        type="button"
        class="settings__danger"
        onclick={confirmReset}
        disabled={resetInFlight}
      >
        {resetInFlight ? 'Resetting…' : 'Reset'}
      </button>
    </div>
  </div>
{/if}

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

  .settings__hint a {
    color: var(--color-primary);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .settings__hint a:hover {
    color: color-mix(in oklab, var(--color-primary) 85%, white);
  }

  .settings__hint a:focus-visible {
    outline: 2px solid var(--color-primary);
    outline-offset: 2px;
    border-radius: var(--radius-xs);
  }

  .settings__hint--inline {
    margin: var(--space-xs) 0 var(--space-sm) 0;
  }

  .settings__hint--field,
  .settings__field > .settings__hint {
    grid-column: 2;
    margin-top: calc(var(--space-xs) * -1);
  }

  /* ----- Form scaffolding (Audio / Graphics / Storage) ----- */

  .settings__form {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
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
    width: 100%;
    max-width: 420px;
    min-width: 0;
    flex-wrap: nowrap;
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

  /* ----- DJ / Master Bus status pill & cue-bleed warning ----- */

  .settings__dj-badge {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-body-xs);
    padding: 2px var(--space-sm);
    border-radius: var(--radius-full);
    border: 1px solid var(--color-line-2);
    background: var(--color-bg-2);
    color: var(--color-text-1);
    white-space: nowrap;
  }

  .settings__dj-badge--recognized {
    background: var(--color-primary);
    color: var(--color-on-primary);
    border-color: var(--color-primary);
  }

  .settings__dj-warn {
    margin: 0;
    justify-self: start;
    display: inline-flex;
    align-items: center;
    padding: 2px var(--space-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-danger, #d04a4a);
    background: var(--color-bg-2);
    color: var(--color-text-0);
    font-size: var(--font-size-body-xs);
    line-height: 1.35;
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

  /* ----- Wave 2.7 Lane SET.D: Reset-to-Defaults button + confirm modal ----- */
  .settings__danger {
    background: var(--color-bg-2);
    color: var(--color-danger, #d04a4a);
    border: 1px solid var(--color-danger, #d04a4a);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-md);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
  }
  .settings__danger:hover:not(:disabled) {
    background: var(--color-danger, #d04a4a);
    color: var(--color-bg-0, #fff);
  }
  .settings__danger:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .settings__modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    z-index: 100;
  }

  .settings__modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 101;
    background: var(--color-bg-1);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-md, 8px);
    padding: var(--space-lg) var(--space-xl);
    width: min(440px, calc(100vw - 2 * var(--space-xl)));
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  }

  .settings__modal-title {
    margin: 0 0 var(--space-md) 0;
    font-size: var(--font-size-display-sm);
    font-weight: 600;
  }

  .settings__modal-body {
    margin: 0 0 var(--space-lg) 0;
    color: var(--color-text-1);
    font-size: var(--font-size-body-sm);
    line-height: 1.5;
  }

  .settings__modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-sm);
  }

  .settings__modal-actions button {
    background: var(--color-bg-2);
    color: var(--color-text-0);
    border: 1px solid var(--color-line-2);
    border-radius: var(--radius-sm);
    padding: var(--space-xs) var(--space-md);
    font-size: var(--font-size-body-sm);
    cursor: pointer;
  }
  .settings__modal-actions button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
