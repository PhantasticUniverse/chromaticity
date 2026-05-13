// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane C.3 operator-settings singleton store
// (`/Users/xaviermac/.claude/plans/inherited-percolating-quill.md`).
//
// Mounts the persisted settings document via the `get_settings`
// Tauri IPC handler (`app/shell/src/commands/settings.rs`) and
// debounces writes back via `set_settings`. Settings.svelte (and
// any future consumer) binds form fields to `settingsStore.snapshot`
// and calls `settingsStore.update({ ... })` to mutate; the store
// schedules a single debounced write-through per the supplied
// `DEBOUNCE_MS` window so rapid keystrokes / slider drags don't
// thrash disk I/O.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention;
// the `audioSnapshot.svelte.ts` / `cameraStore.svelte.ts` siblings
// are the live precedents.
//
// **Test posture.** When running outside Tauri (Vitest jsdom with
// no mocked `invoke`, or `npm run dev` vite-only mode) the `load()`
// promise rejects; the store swallows the failure and stays at the
// bundle-faithful defaults so existing Settings.svelte fixture-
// driven tests continue passing without changes. The `set_settings`
// write-through follows the same posture — failures are recorded
// to `error` but don't throw to the caller.

import type { SettingsPayload } from '../ipc/bindings';

/** Debounce window for write-through to disk, in ms. Sized to
 * collapse a typical drag-slider stream (event per frame, ~16ms)
 * into a single save while still feeling responsive. */
const DEBOUNCE_MS = 250;

/**
 * Bundle-faithful default settings document used as the initial
 * `$state` value and as the fallback when `load()` fails (e.g.
 * running outside Tauri). Matches the engine-side
 * `Settings::default()` shape verbatim — the engine remains the
 * source of truth, but duplicating the defaults here avoids
 * blocking the UI mount on the IPC round-trip.
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
    display: 'display-1',
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
  osc: {
    local_ip: '0.0.0.0',
    port: 8000,
    tx_enabled: true,
    rx_enabled: true,
  },
};

/** Shallow-clone the defaults POJO for the initial `$state` value
 * and for `reset()`. `structuredClone` is overkill (DEFAULT_SETTINGS
 * is a plain object literal with primitives + arrays of primitives)
 * and triggers a DataCloneError when handed a Svelte 5 `$state`
 * proxy elsewhere. */
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
    osc: { ...DEFAULT_SETTINGS.osc },
  };
}

class SettingsStore {
  /** Current in-memory settings document. Initial value is the
   * bundle-faithful default; replaced verbatim by the first
   * successful `load()` resolution. */
  snapshot = $state<SettingsPayload>(cloneDefaults());

  /** Track whether load() has completed (success or failure). The
   * Settings view can render a skeleton state pre-load if desired;
   * the bundle-faithful default snapshot is always displayable, so
   * the W21 PR1 form scaffolding renders unchanged. */
  isLoaded = $state(false);

  /** Tracks the in-flight load() promise (idempotency latch). */
  #loadPromise: Promise<void> | null = null;

  /** Last load() / set() error surface, or null on success. Drives
   * an optional operator-facing banner. */
  error = $state<string | null>(null);

  /** Pending debounce timer for the write-through. */
  #saveTimer: ReturnType<typeof setTimeout> | null = null;

  /**
   * Read the persisted settings document from the shell and replace
   * the in-memory snapshot. Idempotent — subsequent calls return
   * the same settled promise so view-mount `$effect`s can call
   * this unconditionally.
   *
   * Failure mode: when the IPC import or invocation rejects (test
   * harness with no mock, vite dev with no shell), the snapshot
   * stays at the bundle-faithful default and `error` records the
   * failure message. The store still resolves so callers' awaits
   * complete.
   */
  load(): Promise<void> {
    if (this.#loadPromise) return this.#loadPromise;
    this.#loadPromise = this.#doLoad();
    return this.#loadPromise;
  }

  async #doLoad(): Promise<void> {
    try {
      const { getSettings } = await import('../ipc/settings');
      const fresh = await getSettings();
      // Guard against a malformed IPC response (mocked invoke
      // returning `undefined`, or a future shape divergence between
      // the shell and the bridge). The snapshot stays at the
      // bundle-faithful default when the response is not a usable
      // SettingsPayload; the error surface records the issue.
      if (fresh && typeof fresh === 'object' && 'audio' in fresh && 'graphics' in fresh) {
        this.snapshot = fresh;
        this.error = null;
      } else {
        this.error = 'get_settings returned a malformed payload';
      }
    } catch (e) {
      // Swallow — no Tauri runtime available; snapshot stays at
      // the bundle-faithful default and the view renders normally.
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.isLoaded = true;
    }
  }

  /**
   * Apply a partial update to the snapshot and schedule a debounced
   * write-through. The patch shape mirrors `Partial<SettingsPayload>`
   * but with section-level granularity — pass `{ audio: { ... } }`
   * to replace the audio section, or `{ general: { theme: 'Atelier' } }`
   * to update a leaf.
   *
   * Sub-section patches are shallow-merged: if you pass
   * `{ audio: { sample_rate_hz: 96000 } }` the other audio fields
   * retain their current values.
   */
  update(patch: SettingsPatch): void {
    const next: SettingsPayload = {
      general: { ...this.snapshot.general, ...(patch.general ?? {}) },
      audio: { ...this.snapshot.audio, ...(patch.audio ?? {}) },
      graphics: { ...this.snapshot.graphics, ...(patch.graphics ?? {}) },
      storage: { ...this.snapshot.storage, ...(patch.storage ?? {}) },
      osc: { ...this.snapshot.osc, ...(patch.osc ?? {}) },
    };
    this.snapshot = next;
    this.#scheduleSave();
  }

  #scheduleSave(): void {
    if (this.#saveTimer !== null) {
      clearTimeout(this.#saveTimer);
    }
    this.#saveTimer = setTimeout(() => {
      this.#saveTimer = null;
      void this.#doSave();
    }, DEBOUNCE_MS);
  }

  async #doSave(): Promise<void> {
    try {
      const { setSettings } = await import('../ipc/settings');
      // Snapshot a structural copy so an in-flight save doesn't
      // race a concurrent mutation — `setSettings` carries the
      // payload to disk; the next update() iteration will pick up
      // any newer state via its own debounced save. `$state.snapshot`
      // unwraps the Svelte 5 reactive proxy into a plain POJO suitable
      // for `structuredClone` (Tauri's IPC serializer + jsdom's
      // postMessage equivalent both choke on the raw proxy).
      const plain = $state.snapshot(this.snapshot) as SettingsPayload;
      await setSettings(plain);
      this.error = null;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  /**
   * Force an immediate write-through, bypassing the debounce. Used
   * by the Settings view when the user takes an action whose
   * persistence shouldn't wait (e.g. clicking "Set" on a tap-tempo
   * row). Awaits the in-flight save so callers can chain on
   * completion.
   */
  async flush(): Promise<void> {
    if (this.#saveTimer !== null) {
      clearTimeout(this.#saveTimer);
      this.#saveTimer = null;
    }
    await this.#doSave();
  }

  /**
   * Reset persisted settings to engine defaults.
   *
   * Wave 2.7 Lane SET.D: invokes the `reset_settings` Tauri command
   * which swaps the in-memory document to `Settings::default()` AND
   * writes the default document through to the on-disk TOML file.
   * Hydrates the local snapshot from the returned payload so the
   * Settings view re-renders without a follow-on `load()` round-trip.
   *
   * Cancels any pending debounced save (the in-flight write would
   * race the reset write and clobber it). Errors are recorded onto
   * `error` and the snapshot stays at its current state so the UI
   * can surface a banner without the reset half-applying.
   */
  async resetToDefaults(): Promise<void> {
    if (this.#saveTimer !== null) {
      clearTimeout(this.#saveTimer);
      this.#saveTimer = null;
    }
    try {
      const { resetSettings } = await import('../ipc/settings');
      const fresh = await resetSettings();
      if (fresh && typeof fresh === 'object' && 'audio' in fresh && 'graphics' in fresh) {
        this.snapshot = fresh;
        this.error = null;
      } else {
        this.error = 'reset_settings returned a malformed payload';
      }
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  /**
   * Test-harness reset. Clears the snapshot to defaults, cancels
   * the pending save, drops the load-idempotency latch.
   */
  reset(): void {
    if (this.#saveTimer !== null) {
      clearTimeout(this.#saveTimer);
      this.#saveTimer = null;
    }
    this.snapshot = cloneDefaults();
    this.isLoaded = false;
    this.error = null;
    this.#loadPromise = null;
  }
}

/** Section-level patch shape. Pass a subset of any section to
 * patch only that section's fields; missing sections leave the
 * current snapshot's section untouched. */
export interface SettingsPatch {
  general?: Partial<SettingsPayload['general']>;
  audio?: Partial<SettingsPayload['audio']>;
  graphics?: Partial<SettingsPayload['graphics']>;
  storage?: Partial<SettingsPayload['storage']>;
  osc?: Partial<SettingsPayload['osc']>;
}

export const settingsStore = new SettingsStore();
