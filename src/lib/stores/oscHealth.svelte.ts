// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.4 Lane PERF.D OSC health polling store. Backs the TopBar
// adapter pill (`OSC` kind) — `start()` schedules a 5 s polling loop
// that invokes the `osc_health_snapshot` Tauri command shipped by
// Wave 2.1 D.3 (`app/shell/src/commands/osc.rs`) and stores the latest
// snapshot.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention; the
// `audioSnapshot.svelte.ts` + `midiStore.svelte.ts` siblings are the
// live precedents for this shape (class-instance singleton +
// module-level export).
//
// **Dual-mode posture (no-Tauri-context fallback).** When running
// outside Tauri (vite dev / Vitest jsdom), the dynamic import or the
// `invoke()` call rejects; the store records the error onto `error`
// and leaves `snapshot` at its prior value. Consumers fall back to
// the off / unknown pill state when `snapshot` is `null`.
//
// `start()` is idempotent: subsequent calls return without scheduling
// a duplicate timer. `stop()` clears the timer and is safe to call
// repeatedly. `reset()` clears state for test harnesses.

import type { OscHealthSnapshot } from '../ipc/bindings';

class OscHealthStore {
  /** Latest health snapshot from the engine, or `null` before the
   * first poll (or after `reset()`). */
  snapshot: OscHealthSnapshot | null = $state(null);

  /** Last poll error message, or `null` on success / fresh state. */
  error: string | null = $state(null);

  /** Active polling timer handle, or `null` when stopped. */
  #interval: ReturnType<typeof setInterval> | null = null;

  /**
   * Invoke `osc_health_snapshot` once and update `snapshot` / `error`.
   * Called internally by the polling loop; exposed for test harnesses
   * + first-call eager fetch from `start()`.
   */
  async refresh(): Promise<void> {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      this.snapshot = await invoke<OscHealthSnapshot>('osc_health_snapshot');
      this.error = null;
    } catch (err) {
      this.error = err instanceof Error ? err.message : String(err);
    }
  }

  /**
   * Start the polling loop. Fires `refresh()` immediately then every
   * `intervalMs` (defaults to 5000 — the Wave 2.4 D.D brief cadence).
   * Idempotent: a second call while the timer is active returns
   * without scheduling a duplicate.
   */
  start(intervalMs = 5000): void {
    if (this.#interval) return;
    void this.refresh();
    this.#interval = setInterval(() => {
      void this.refresh();
    }, intervalMs);
  }

  /** Halt the polling loop. Safe to call repeatedly. */
  stop(): void {
    if (this.#interval) {
      clearInterval(this.#interval);
      this.#interval = null;
    }
  }

  /** Test-harness reset. Clears snapshot, error, and stops polling. */
  reset(): void {
    this.snapshot = null;
    this.error = null;
    this.stop();
  }
}

export const oscHealthStore = new OscHealthStore();
