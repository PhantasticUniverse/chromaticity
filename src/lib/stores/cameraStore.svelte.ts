// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 1c-β Phase 4 Lane A camera device-picker singleton store.
// Backs the operator-facing camera section in `src/lib/views/Sources.svelte`
// — `refresh()` invokes the new `list_camera_devices` Tauri command
// (`app/shell/src/commands/camera.rs`), `select(deviceId)` records the
// operator's chosen camera, and `loadPersistedSelection()` rehydrates
// the prior selection from `localStorage` on view mount.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention; the
// W33b `capabilities.svelte.ts` + Wave 6 `audioSnapshot.svelte.ts`
// siblings are the live precedents for this shape (class-instance
// singleton + module-level export).
//
// **Phase posture.** The Tauri `list_camera_devices` command returns
// an empty vec at this slice (Phase 2 PR 1 of Wave 1c-β scaffolded
// `NokhwaInputAdapter::enumerate` as a documented stub until the
// per-platform nokhwa enumeration arms wire end-to-end). The store
// surfaces that empty vec verbatim — the consumer view renders the
// "No cameras found" empty-state branch with permission-grant
// guidance copy. Future per-platform follow-up PRs swap the underlying
// adapter implementation with no store / view changes required.
//
// `localStorage`-backed persistence (rather than a Tauri settings store
// command) is the slice-1 expedient: Phase 4 Lane A's operator flow
// only needs cross-tab persistence within a single app launch, and the
// settings-store IPC is post-MVP. The persisted key is the
// platform-stable nokhwa device-id (per ADR-020 §6); reconnect in a
// future enumeration pass matches against this key verbatim.
//
// `subscribe`-style listener detach is intentionally absent: the
// commands surface here is a pull-model (`refresh()` is on demand),
// unlike the audio snapshot store that listens to a push-model Tauri
// event channel.

import { invoke } from '@tauri-apps/api/core';

import type { CameraDevicePayload } from '../ipc/bindings/CameraDevicePayload';

/** localStorage key for the operator's last-selected camera device id.
 * Namespaced under `chromaticity.camera.` so a future settings-store
 * migration can scan the prefix when porting persisted state. */
const PERSISTED_DEVICE_KEY = 'chromaticity.camera.selectedDevice';

/** Tauri command name surfaced by `app/shell/src/commands/camera.rs`. */
const LIST_CAMERA_DEVICES_COMMAND = 'list_camera_devices';

class CameraStore {
  /** Latest enumeration result. Empty vec under the Phase-4 Lane A stub
   * posture; non-empty once the per-platform nokhwa enumeration arms
   * wire end-to-end. */
  devices = $state<readonly CameraDevicePayload[]>([]);

  /** Operator-selected device id, or `null` if none has been chosen
   * (initial state, or after explicit deselect). Persisted across
   * sessions via `localStorage` per the slice-1 narrowing. */
  selected = $state<string | null>(null);

  /** Tracks the in-flight `refresh()` promise so the consumer view can
   * render a loading spinner without dispatching duplicate enumeration
   * calls. */
  isLoading = $state(false);

  /** Last refresh error surface, or `null` on success / fresh state.
   * Drives the operator-facing error banner when the command fails
   * (e.g. running outside Tauri, or a future backend surfacing a
   * per-platform fault). */
  error = $state<string | null>(null);

  /** Invoke `list_camera_devices` and replace the device list. Sets
   * `isLoading` for the duration; on failure, clears the device list
   * and records the error. Idempotent on the failure path — callers
   * can retry without first clearing state. */
  async refresh(): Promise<void> {
    this.isLoading = true;
    this.error = null;
    try {
      const fresh = await invoke<CameraDevicePayload[]>(LIST_CAMERA_DEVICES_COMMAND);
      this.devices = fresh;
    } catch (e) {
      this.devices = [];
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.isLoading = false;
    }
  }

  /** Record the operator's selected device id (or clear the selection
   * with `null`) and persist it to `localStorage`. The persistence
   * write tolerates browser storage exceptions silently — operators
   * running in private-browsing modes still get the in-memory
   * selection, just without cross-launch durability.
   *
   * `localStorage` may not exist (SSR-style harness, server-side
   * import); the `typeof` guard tolerates that without throwing. */
  select(deviceId: string | null): void {
    this.selected = deviceId;
    if (typeof localStorage === 'undefined') return;
    try {
      if (deviceId) {
        localStorage.setItem(PERSISTED_DEVICE_KEY, deviceId);
      } else {
        localStorage.removeItem(PERSISTED_DEVICE_KEY);
      }
    } catch {
      // Swallow — storage quota / private-mode exceptions leave the
      // in-memory selection intact, which is the operator-friendly
      // failure mode.
    }
  }

  /** Read the persisted device id from `localStorage` (if present) and
   * apply it to `selected` without firing a `setItem` round-trip. The
   * `typeof` guard tolerates SSR-style harnesses; storage-read
   * exceptions are swallowed — the operator simply starts with no
   * selection. */
  loadPersistedSelection(): void {
    if (typeof localStorage === 'undefined') return;
    try {
      const persisted = localStorage.getItem(PERSISTED_DEVICE_KEY);
      if (persisted) this.selected = persisted;
    } catch {
      // Swallow — no persisted selection is the safe default.
    }
  }

  /** Test-harness reset. Clears the device list, selection, loading,
   * and error state; does NOT touch `localStorage` (callers manage
   * persistence test cleanup directly to keep failure-mode tests
   * deterministic). */
  reset(): void {
    this.devices = [];
    this.selected = null;
    this.isLoading = false;
    this.error = null;
  }
}

export const cameraStore = new CameraStore();
