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
// to `error` but don't throw to the caller. Settings.svelte renders
// `error` as a persist-failure banner gated on `canPersist` (true
// only after a real shell load), so the expected no-Tauri load
// failure never surfaces as operator-facing noise.
//
// **Pre-load updates.** `update()` calls made before the first
// successful `get_settings` (the `canPersist` window) accumulate
// into a preload patch that is re-applied on top of the loaded
// snapshot and then persisted — a change made in the first moments
// after launch survives the load instead of being silently
// clobbered and reverted.

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
  live_sources: {
    syphon_input_enabled: true,
    syphon_output_enabled: true,
    ndi_input_enabled: false,
    ndi_output_enabled: true,
    constant_output_resolution: false,
    sender_name: 'chrm',
  },
  osc: {
    // ADR-013 §1: loopback-only by default; routable OSC bind is an
    // explicit operator opt-in.
    local_ip: '127.0.0.1',
    port: 9000,
    out_host: '127.0.0.1',
    out_port: 9001,
    tx_enabled: true,
    rx_enabled: true,
    // Compat dual-emit window (parity-closure wave 2 Lane D PR10).
    // Off by default; operator opts in for legacy-controller migrations.
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
    live_sources: { ...DEFAULT_SETTINGS.live_sources },
    osc: { ...DEFAULT_SETTINGS.osc },
    tempo: {
      ...DEFAULT_SETTINGS.tempo,
      bpm_source_priority: [...DEFAULT_SETTINGS.tempo.bpm_source_priority],
    },
  };
}

/** Section-level shallow merge of a patch onto a snapshot. Shared by
 * `update()`, `seedHint()`, and the post-load preload-patch replay so
 * all three apply identical merge semantics. */
function mergePatch(current: SettingsPayload, patch: SettingsPatch): SettingsPayload {
  return {
    general: { ...current.general, ...(patch.general ?? {}) },
    audio: { ...current.audio, ...(patch.audio ?? {}) },
    graphics: { ...current.graphics, ...(patch.graphics ?? {}) },
    storage: { ...current.storage, ...(patch.storage ?? {}) },
    live_sources: { ...current.live_sources, ...(patch.live_sources ?? {}) },
    osc: { ...current.osc, ...(patch.osc ?? {}) },
    tempo: { ...current.tempo, ...(patch.tempo ?? {}) },
  };
}

/** Accumulate `patch` onto `base` with the same section-level shallow
 * merge `update()` uses, so a sequence of pre-load patches collapses
 * into one patch that replays in order. */
function mergePatches(base: SettingsPatch, patch: SettingsPatch): SettingsPatch {
  const next: SettingsPatch = { ...base };
  if (patch.general) next.general = { ...base.general, ...patch.general };
  if (patch.audio) next.audio = { ...base.audio, ...patch.audio };
  if (patch.graphics) next.graphics = { ...base.graphics, ...patch.graphics };
  if (patch.storage) next.storage = { ...base.storage, ...patch.storage };
  if (patch.live_sources) next.live_sources = { ...base.live_sources, ...patch.live_sources };
  if (patch.osc) next.osc = { ...base.osc, ...patch.osc };
  if (patch.tempo) next.tempo = { ...base.tempo, ...patch.tempo };
  return next;
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

  /** True only after `get_settings` returns a valid shell payload.
   * Debounced saves are gated on this so pre-load fixture seeding
   * cannot overwrite a persisted settings file. */
  canPersist = $state(false);

  /** Tracks the in-flight load() promise (idempotency latch). */
  #loadPromise: Promise<void> | null = null;

  /** Last load() / set() error surface, or null on success. Rendered
   * by Settings.svelte as a persist-failure banner gated on
   * `canPersist` (a save can only fail after a real shell load, so
   * the expected no-Tauri load failure stays silent). */
  error = $state<string | null>(null);

  /** Pending debounce timer for the write-through. */
  #saveTimer: ReturnType<typeof setTimeout> | null = null;

  /** Patches applied via `update()` before the first successful
   * `get_settings`. Re-applied on top of the loaded snapshot (then
   * persisted) so an operator change made before `canPersist` flips
   * survives the load instead of being silently reverted. */
  #preloadPatch: SettingsPatch | null = null;

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
        this.canPersist = true;
        this.error = null;
        // Re-apply any update() made before the load resolved on top
        // of the authoritative disk snapshot, then persist it — the
        // pre-load change survives instead of silently reverting.
        const pending = this.#preloadPatch;
        this.#preloadPatch = null;
        if (pending) {
          this.snapshot = mergePatch(this.snapshot, pending);
          this.#scheduleSave();
        }
      } else {
        this.canPersist = false;
        this.error = 'get_settings returned a malformed payload';
      }
    } catch (e) {
      // Swallow — no Tauri runtime available; snapshot stays at
      // the bundle-faithful default and the view renders normally.
      this.canPersist = false;
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
    this.snapshot = mergePatch(this.snapshot, patch);
    if (this.isLoaded && this.canPersist) {
      this.#scheduleSave();
    } else if (!this.canPersist) {
      // Pre-load (or load-failed) window: record the patch so a
      // successful load can re-apply it on top of the disk snapshot
      // instead of clobbering the operator's change.
      this.#preloadPatch = mergePatches(this.#preloadPatch ?? {}, patch);
    }
  }

  /**
   * Hint-only snapshot seed for pre-load paint fast-paths (the
   * localStorage theme hint in `src/lib/theme.ts`). Merges like
   * `update()` but schedules no save and records no preload patch,
   * so the authoritative `load()` snapshot always wins — a stale
   * hint can never be persisted over the fresher disk value.
   */
  seedHint(patch: SettingsPatch): void {
    this.snapshot = mergePatch(this.snapshot, patch);
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
    if (!this.isLoaded || !this.canPersist) {
      return;
    }
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
   * Force an immediate write-through, bypassing the debounce. Wired
   * to this module's `beforeunload` listener so a change made within
   * the debounce window isn't silently dropped when the window
   * closes (the pending `setTimeout` dies with the webview). Awaits
   * the in-flight save so non-exit callers can chain on completion.
   */
  async flush(): Promise<void> {
    if (this.#saveTimer !== null) {
      clearTimeout(this.#saveTimer);
      this.#saveTimer = null;
    }
    if (this.isLoaded && this.canPersist) {
      await this.#doSave();
    }
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
    // The operator explicitly chose the defaults; drop any pre-load
    // patch so it cannot replay over the reset document.
    this.#preloadPatch = null;
    try {
      const { resetSettings } = await import('../ipc/settings');
      const fresh = await resetSettings();
      if (fresh && typeof fresh === 'object' && 'audio' in fresh && 'graphics' in fresh) {
        this.snapshot = fresh;
        this.isLoaded = true;
        this.canPersist = true;
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
    this.canPersist = false;
    this.error = null;
    this.#loadPromise = null;
    this.#preloadPatch = null;
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
  live_sources?: Partial<SettingsPayload['live_sources']>;
  osc?: Partial<SettingsPayload['osc']>;
  tempo?: Partial<SettingsPayload['tempo']>;
}

export const settingsStore = new SettingsStore();

// Flush any pending debounced write before the document unloads so a
// change made within the DEBOUNCE_MS window isn't lost on exit. The
// Tauri webview fires `beforeunload` on window close as well, so this
// covers the desktop quit path. Best-effort by nature: the listener
// cannot await the async IPC, but kicking the save off synchronously
// gives the write-through its only chance to reach the shell (the
// `ipc/settings` module is already import-cached after any prior
// save, so the dynamic import resolves without a network/disk hop).
if (typeof window !== 'undefined') {
  window.addEventListener('beforeunload', () => {
    void settingsStore.flush();
  });
}
