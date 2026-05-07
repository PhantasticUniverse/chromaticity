// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 1c-β Phase 4 Lane B Syphon source-picker + output-publish
// singleton store. Backs the operator-facing Syphon section in
// `src/lib/views/Sources.svelte` — `refreshServers()` invokes
// `list_syphon_servers` (`app/shell/src/commands/syphon.rs`),
// `publishOutput(name)` / `unpublishOutput()` invoke the matching
// publish / unpublish Tauri commands, and `loadPersistedSelection()`
// rehydrates the prior input selection from `localStorage` on view
// mount.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention;
// the Phase 4 Lane A `cameraStore.svelte.ts` sibling is the direct
// shape precedent (class-instance singleton + module-level export).
//
// **Phase posture.** The `list_syphon_servers` Tauri command returns
// an empty vec under the Phase 3 PR 2 `SyphonServerDirectory` stub
// (the live `NSDistributedNotificationCenter` observer wire requires
// the Tauri main-thread runloop per `docs/54` Phase-3 lesson 4 and
// lands in a follow-up wave). The publish / unpublish commands ship
// synthetic `SyphonOutputStatusPayload` values until the
// `SyphonClientHandle: Send` vs `Retained<>: !Send` tension resolves
// in Phase 5. The store surfaces these stub values verbatim — the
// view renders the "no Syphon sources found" empty-state branch and
// the trust-boundary warning fires off the synthetic active-status
// flag, so the operator-facing safety surface is in place
// independent of the underlying registration.
//
// **CRITICAL trust-boundary contract** per ADR-021 §10 +
// `docs/29-cleanroom-legal-policy-baseline.md` Wave 1c-β threat-model
// addendum: when `output.active === true`, the consumer view MUST
// display a non-dismissible warning banner with the canonical copy
// "Syphon output is visible to any application on this machine. Do
// not publish surfaces containing PII or secrets." The store does not
// enforce this directly — it surfaces the active flag through reactive
// state and the consumer view drives the non-dismissible banner
// rendering. See `Sources.svelte` for the rendering site.
//
// Default-off output posture per ADR-013 + ADR-016 default-off
// telemetry precedent: the initial `output` state is
// `{active: false, server_name: null}` and only flips to `active:
// true` after explicit operator opt-in via the publish toggle.
//
// `localStorage`-backed input persistence (rather than a Tauri
// settings store command) mirrors the Phase 4 Lane A camera-store
// expedient: cross-tab persistence within a single app launch is the
// only requirement; settings-store IPC is post-MVP. The persisted key
// is the framework-stable Syphon server UUID per ADR-021 §4.

import { invoke } from '@tauri-apps/api/core';

import type { SyphonOutputStatusPayload } from '../ipc/bindings/SyphonOutputStatusPayload';
import type { SyphonServerPayload } from '../ipc/bindings/SyphonServerPayload';

/** localStorage key for the operator's last-selected Syphon server
 * UUID. Namespaced under `chromaticity.syphon.` so a future settings-
 * store migration can scan the prefix when porting persisted state. */
const PERSISTED_INPUT_KEY = 'chromaticity.syphon.selectedInput';

/** Tauri command names surfaced by `app/shell/src/commands/syphon.rs`. */
const LIST_SYPHON_SERVERS_COMMAND = 'list_syphon_servers';
const PUBLISH_SYPHON_OUTPUT_COMMAND = 'publish_syphon_output';
const UNPUBLISH_SYPHON_OUTPUT_COMMAND = 'unpublish_syphon_output';

/** Default operator-facing output server name. Mirrors
 * `chromaticity_syphon_adapter::DEFAULT_OUTPUT_NAME_TEMPLATE` so the
 * frontend pre-fills the publish-name input field with a sensible
 * default that the operator can override before opting in. */
export const DEFAULT_OUTPUT_NAME = 'Chromaticity Performance';

class SyphonStore {
  /** Latest enumeration result. Empty vec under the Phase-4 Lane B
   * stub posture; non-empty once the live
   * `NSDistributedNotificationCenter` observer wires end-to-end. */
  servers = $state<readonly SyphonServerPayload[]>([]);

  /** Operator-selected input server UUID, or `null` if none has been
   * chosen (initial state, or after explicit deselect). Persisted
   * across sessions via `localStorage` per the slice-1 narrowing. */
  selectedInput = $state<string | null>(null);

  /** Output-publish status. Default-off per ADR-013 / ADR-016 — the
   * initial state is inactive until the operator explicitly opts in
   * via the publish toggle. Drives the non-dismissible trust-boundary
   * warning banner in the consumer view per ADR-021 §10. */
  output = $state<SyphonOutputStatusPayload>({ active: false, server_name: null });

  /** Tracks the in-flight `refreshServers()` promise so the consumer
   * view can render a loading spinner without dispatching duplicate
   * enumeration calls. */
  isLoading = $state(false);

  /** Last refresh / publish / unpublish error surface, or `null` on
   * success / fresh state. Drives the operator-facing error banner
   * when a command fails (e.g. running outside Tauri, non-macOS
   * platform, future backend surfacing a per-platform fault). */
  error = $state<string | null>(null);

  /** Invoke `list_syphon_servers` and replace the server list. Sets
   * `isLoading` for the duration; on failure, clears the server list
   * and records the error. Idempotent on the failure path — callers
   * can retry without first clearing state. */
  async refreshServers(): Promise<void> {
    this.isLoading = true;
    this.error = null;
    try {
      const fresh = await invoke<SyphonServerPayload[]>(LIST_SYPHON_SERVERS_COMMAND);
      this.servers = fresh;
    } catch (e) {
      this.servers = [];
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.isLoading = false;
    }
  }

  /** Invoke `publish_syphon_output` with the operator-supplied server
   * name and update the `output` state with the returned status. On
   * failure the output state stays at its prior value (the publish
   * attempt did not transition the framework state) and the error is
   * recorded for the consumer banner.
   *
   * Per ADR-021 §10: the consumer view MUST render the non-
   * dismissible trust-boundary warning whenever this returns a status
   * with `active: true`. */
  async publishOutput(name: string): Promise<void> {
    this.error = null;
    try {
      const status = await invoke<SyphonOutputStatusPayload>(PUBLISH_SYPHON_OUTPUT_COMMAND, {
        name,
      });
      this.output = status;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  /** Invoke `unpublish_syphon_output` and update the `output` state
   * with the returned (inactive) status. On failure the output state
   * stays at its prior value and the error is recorded for the
   * consumer banner. */
  async unpublishOutput(): Promise<void> {
    this.error = null;
    try {
      const status = await invoke<SyphonOutputStatusPayload>(UNPUBLISH_SYPHON_OUTPUT_COMMAND);
      this.output = status;
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  /** Record the operator's selected input server UUID (or clear the
   * selection with `null`) and persist it to `localStorage`. The
   * persistence write tolerates browser storage exceptions silently —
   * operators running in private-browsing modes still get the
   * in-memory selection, just without cross-launch durability.
   *
   * `localStorage` may not exist (SSR-style harness, server-side
   * import); the `typeof` guard tolerates that without throwing. */
  selectInput(uuid: string | null): void {
    this.selectedInput = uuid;
    if (typeof localStorage === 'undefined') return;
    try {
      if (uuid) {
        localStorage.setItem(PERSISTED_INPUT_KEY, uuid);
      } else {
        localStorage.removeItem(PERSISTED_INPUT_KEY);
      }
    } catch {
      // Swallow — storage quota / private-mode exceptions leave the
      // in-memory selection intact, which is the operator-friendly
      // failure mode.
    }
  }

  /** Read the persisted input UUID from `localStorage` (if present)
   * and apply it to `selectedInput` without firing a `setItem` round-
   * trip. The `typeof` guard tolerates SSR-style harnesses; storage-
   * read exceptions are swallowed — the operator simply starts with
   * no selection. */
  loadPersistedSelection(): void {
    if (typeof localStorage === 'undefined') return;
    try {
      const persisted = localStorage.getItem(PERSISTED_INPUT_KEY);
      if (persisted) this.selectedInput = persisted;
    } catch {
      // Swallow — no persisted selection is the safe default.
    }
  }

  /** Test-harness reset. Clears the server list, selection, output
   * status, loading, and error state; does NOT touch `localStorage`
   * (callers manage persistence test cleanup directly to keep
   * failure-mode tests deterministic). */
  reset(): void {
    this.servers = [];
    this.selectedInput = null;
    this.output = { active: false, server_name: null };
    this.isLoading = false;
    this.error = null;
  }
}

export const syphonStore = new SyphonStore();
