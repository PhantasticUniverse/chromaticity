// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Wave 2.1 Lane C.3 operator-settings singleton store, upgraded to an
// explicit discriminated state machine (`SettingsMachine`).
//
// State transitions:
//   unhydrated ── load() ok ──────────────▶ clean
//   unhydrated ── load() err/malformed ───▶ offline (in-memory edits only)
//   clean      ── update() ───────────────▶ dirty (debounce timer armed)
//   dirty      ── timer / flush() ────────▶ saving
//   saving     ── update() ───────────────▶ saving (dirtyWhileSaving = true)
//   saving     ── save ok (clean) ────────▶ clean
//   saving     ── save ok (dirty) ────────▶ dirty (re-arms debounce)
//   saving     ── save err ───────────────▶ clean (lastError recorded)
//   any        ── resetToDefaults() ──────▶ resetting ──▶ clean (or prior + err)
//
// Back-compat `$derived` getters (`isLoaded`, `canPersist`, `error`) preserve
// the exact public API so all views and tests continue working unchanged.

import { untrack } from 'svelte';

import type { SettingsPayload } from '../ipc/bindings';
import { formatErrorMessage, registerStateMachine } from '../state';

/** Debounce window for write-through to disk, in ms. */
const DEBOUNCE_MS = 250;

/** Explicit lifecycle state of `SettingsStore`. */
export type SettingsMachine =
  | { readonly kind: 'unhydrated' }
  | { readonly kind: 'offline'; readonly error: string }
  | { readonly kind: 'clean'; readonly lastError: string | null }
  | { readonly kind: 'dirty'; readonly lastError: string | null }
  | { readonly kind: 'saving'; readonly dirtyWhileSaving: boolean }
  | { readonly kind: 'resetting' };

/**
 * Bundle-faithful default settings document used as the initial
 * `$state` value and as the fallback when `load()` fails (e.g.
 * running outside Tauri). Matches `Settings::default()` verbatim.
 */
export const DEFAULT_SETTINGS: SettingsPayload = {
  general: {
    theme: 'Pulse',
  },
  audio: {
    input_device: 'default',
    output_device: 'default',
    sample_rate_hz: 48_000,
    latency_hint: 'balanced',
    channel_layout: 'stereo',
    channel_left: 'L1',
    channel_right: 'R2',
    input_gain: 0.59,
  },
  graphics: {
    resolution_preset: '1080p',
    custom_resolution_enabled: false,
    custom_width: 1920,
    custom_height: 1080,
    fps_target: '60',
    vsync: true,
    hdr_target: 'sdr',
    display: 'auto',
    color_space: 'srgb',
    multi_display_mode: 'single',
    render_scale: 1.0,
    screen_fitting: 'fit',
    fps_slider_enabled: false,
  },
  storage: {
    media_folders: [],
    scene_folders: [],
    backup_path: '',
    backup_retention: 'retain-10',
  },
  live_sources: {
    syphon_input_enabled: true,
    syphon_output_enabled: true,
    ndi_input_enabled: false,
    ndi_output_enabled: true,
    constant_output_resolution: false,
    sender_name: 'chrm',
  },
  osc: {
    local_ip: '127.0.0.1',
    port: 9000,
    out_host: '127.0.0.1',
    out_port: 9001,
    tx_enabled: true,
    rx_enabled: true,
    dual_emit_enabled: false,
    dual_emit_until: null,
  },
  tempo: {
    ableton_link_enabled: true,
    start_stop_sync: true,
    midi_clock_in_enabled: false,
    midi_clock_in_device: 'any',
    midi_clock_out_enabled: true,
    midi_clock_out_device: 'iac-1',
    quantize: 'bar',
    manual_bpm: 120.0,
    link_offset_ms: 0,
    audio_bpm_sensitivity: 60,
    bpm_source_priority: ['audio', 'midiclock', 'link', 'manual'],
  },
};

function cloneDefaults(): SettingsPayload {
  return {
    general: { ...DEFAULT_SETTINGS.general },
    audio: { ...DEFAULT_SETTINGS.audio },
    graphics: { ...DEFAULT_SETTINGS.graphics },
    storage: {
      ...DEFAULT_SETTINGS.storage,
      media_folders: [...DEFAULT_SETTINGS.storage.media_folders],
      scene_folders: [...DEFAULT_SETTINGS.storage.scene_folders],
    },
    live_sources: { ...DEFAULT_SETTINGS.live_sources },
    osc: { ...DEFAULT_SETTINGS.osc },
    tempo: {
      ...DEFAULT_SETTINGS.tempo,
      bpm_source_priority: [...DEFAULT_SETTINGS.tempo.bpm_source_priority],
    },
  };
}

function isValidSettingsPayload(value: unknown): value is SettingsPayload {
  return Boolean(value && typeof value === 'object' && 'audio' in value && 'graphics' in value);
}

class SettingsStore {
  /** Current in-memory settings document. */
  snapshot = $state<SettingsPayload>(cloneDefaults());

  /** Explicit discriminated lifecycle state. */
  machine = $state<SettingsMachine>({ kind: 'unhydrated' });

  /** Monotonic generation counter to drop stale async completions after reset(). */
  #generation = 0;

  /** Tracks the in-flight load() promise (idempotency latch). */
  #loadPromise: Promise<void> | null = null;

  /** Pending debounce timer for the write-through. */
  #saveTimer: ReturnType<typeof setTimeout> | null = null;

  /** Back-compat projection: `true` once `load()` has settled (or `resetToDefaults()` succeeded). */
  get isLoaded(): boolean {
    return this.machine.kind !== 'unhydrated';
  }

  /** Back-compat projection: `true` iff hydrated against a live shell and safe to write through. */
  get canPersist(): boolean {
    return this.machine.kind !== 'unhydrated' && this.machine.kind !== 'offline';
  }

  /** Back-compat projection: last load/save/reset error, or `null`. */
  get error(): string | null {
    switch (this.machine.kind) {
      case 'offline':
        return this.machine.error;
      case 'clean':
      case 'dirty':
        return this.machine.lastError;
      case 'unhydrated':
      case 'saving':
      case 'resetting':
        return null;
    }
  }

  load(): Promise<void> {
    if (this.#loadPromise) return this.#loadPromise;
    this.#loadPromise = this.#doLoad(this.#generation);
    return this.#loadPromise;
  }

  async #doLoad(gen: number): Promise<void> {
    try {
      const { getSettings } = await import('../ipc/settings');
      const fresh = await getSettings();
      if (gen !== this.#generation) return;
      if (isValidSettingsPayload(fresh)) {
        this.snapshot = fresh;
        this.machine = { kind: 'clean', lastError: null };
      } else {
        this.machine = { kind: 'offline', error: 'get_settings returned a malformed payload' };
      }
    } catch (e) {
      if (gen !== this.#generation) return;
      this.machine = { kind: 'offline', error: formatErrorMessage(e) };
    }
  }

  update(patch: SettingsPatch): void {
    const snap = untrack(() => this.snapshot);
    const cur = untrack(() => this.machine);
    this.snapshot = {
      general: { ...snap.general, ...(patch.general ?? {}) },
      audio: { ...snap.audio, ...(patch.audio ?? {}) },
      graphics: { ...snap.graphics, ...(patch.graphics ?? {}) },
      storage: { ...snap.storage, ...(patch.storage ?? {}) },
      live_sources: { ...snap.live_sources, ...(patch.live_sources ?? {}) },
      osc: { ...snap.osc, ...(patch.osc ?? {}) },
      tempo: { ...snap.tempo, ...(patch.tempo ?? {}) },
    };

    switch (cur.kind) {
      case 'clean':
      case 'dirty':
        this.machine = { kind: 'dirty', lastError: cur.lastError };
        this.#scheduleSave();
        break;
      case 'saving':
        this.machine = { kind: 'saving', dirtyWhileSaving: true };
        break;
      case 'unhydrated':
      case 'offline':
      case 'resetting':
        // Pre-hydration / offline / resetting: keep in-memory update without scheduling disk I/O.
        break;
    }
  }

  #scheduleSave(): void {
    if (this.#saveTimer !== null) {
      clearTimeout(this.#saveTimer);
    }
    this.#saveTimer = setTimeout(() => {
      this.#saveTimer = null;
      void this.#doSave(this.#generation);
    }, DEBOUNCE_MS);
  }

  async #doSave(gen: number): Promise<void> {
    if (!this.canPersist || this.machine.kind === 'resetting') {
      return;
    }
    this.machine = { kind: 'saving', dirtyWhileSaving: false };
    try {
      const { setSettings } = await import('../ipc/settings');
      const plain = $state.snapshot(this.snapshot) as SettingsPayload;
      await setSettings(plain);
      if (gen !== this.#generation || this.machine.kind !== 'saving') return;
      if (this.machine.dirtyWhileSaving) {
        this.machine = { kind: 'dirty', lastError: null };
        this.#scheduleSave();
      } else {
        this.machine = { kind: 'clean', lastError: null };
      }
    } catch (e) {
      if (gen !== this.#generation || this.machine.kind !== 'saving') return;
      this.machine = { kind: 'clean', lastError: formatErrorMessage(e) };
    }
  }

  async flush(): Promise<void> {
    if (this.#saveTimer !== null) {
      clearTimeout(this.#saveTimer);
      this.#saveTimer = null;
    }
    if (this.canPersist && this.machine.kind !== 'resetting') {
      await this.#doSave(this.#generation);
    }
  }

  async resetToDefaults(): Promise<void> {
    if (this.#saveTimer !== null) {
      clearTimeout(this.#saveTimer);
      this.#saveTimer = null;
    }
    const prior = this.machine;
    const gen = this.#generation;
    this.machine = { kind: 'resetting' };
    try {
      const { resetSettings } = await import('../ipc/settings');
      const fresh = await resetSettings();
      if (gen !== this.#generation) return;
      if (isValidSettingsPayload(fresh)) {
        this.snapshot = fresh;
        this.machine = { kind: 'clean', lastError: null };
      } else {
        const msg = 'reset_settings returned a malformed payload';
        this.machine =
          prior.kind === 'unhydrated' || prior.kind === 'offline'
            ? { kind: 'offline', error: msg }
            : { kind: 'clean', lastError: msg };
      }
    } catch (e) {
      if (gen !== this.#generation) return;
      const msg = formatErrorMessage(e);
      this.machine =
        prior.kind === 'unhydrated' || prior.kind === 'offline'
          ? { kind: 'offline', error: msg }
          : { kind: 'clean', lastError: msg };
    }
  }

  reset(): void {
    this.#generation += 1;
    if (this.#saveTimer !== null) {
      clearTimeout(this.#saveTimer);
      this.#saveTimer = null;
    }
    this.snapshot = cloneDefaults();
    this.machine = { kind: 'unhydrated' };
    this.#loadPromise = null;
  }
}

export interface SettingsPatch {
  general?: Partial<SettingsPayload['general']>;
  audio?: Partial<SettingsPayload['audio']>;
  graphics?: Partial<SettingsPayload['graphics']>;
  storage?: Partial<SettingsPayload['storage']>;
  live_sources?: Partial<SettingsPayload['live_sources']>;
  osc?: Partial<SettingsPayload['osc']>;
  tempo?: Partial<SettingsPayload['tempo']>;
}

export const settingsStore = new SettingsStore();

registerStateMachine('settings', () => ({
  kind: settingsStore.machine.kind,
  canPersist: settingsStore.canPersist,
  error: settingsStore.error,
}));
