// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave 2.1 Lane D.3 OSC route binding singleton store. Backs the
// operator-facing OSC view in `src/lib/views/Osc.svelte` — `refresh()`
// invokes the new `osc_routes_snapshot` Tauri command, `bind()` +
// `unbind()` invoke + refresh.
//
// `.svelte.ts` extension is required for runes-in-module per the
// `feedback_eslint_svelte_dot_ts_parser.md` user-memory convention;
// the W2.1 D.1 `midiStore.svelte.ts` sibling is the live precedent
// for this shape (class-instance singleton + module-level export).
//
// **Phase posture.** The shell-side registry is seeded with the
// canonical UX subset on app bootstrap; running outside Tauri (e.g.
// Vitest jsdom with no mocked `invoke`, vite-only `npm run dev`) the
// refresh / bind / unbind promises reject and the store records the
// error onto `error` while keeping `routes` as whatever fixture-fallback
// the consumer wants to render. The consumer view passes the fixture
// import as the fallback so the empty-out-of-Tauri branch still shows
// the curated routes.

import { invoke } from '@tauri-apps/api/core';

import type { OscRoutePayload } from '../ipc/bindings/OscRoutePayload';

const OSC_ROUTES_SNAPSHOT_COMMAND = 'osc_routes_snapshot';
const BIND_OSC_ROUTE_COMMAND = 'bind_osc_route';
const UNBIND_OSC_ROUTE_COMMAND = 'unbind_osc_route';

class OscStore {
  /** Latest enumeration result. Empty vec when refresh has not yet
   * resolved successfully; non-empty after the first successful
   * refresh against a seeded registry. */
  routes = $state<readonly OscRoutePayload[]>([]);

  /** Tracks the in-flight `refresh()` promise. */
  isLoading = $state(false);

  /** Last refresh / bind / unbind error surface, or `null` on success /
   * fresh state. Drives the operator-facing error banner. */
  error = $state<string | null>(null);

  /** Invoke `osc_routes_snapshot` and replace the route list. On
   * failure, clears the route list and records the error. Idempotent
   * on the failure path. */
  async refresh(): Promise<void> {
    this.isLoading = true;
    this.error = null;
    try {
      const fresh = await invoke<OscRoutePayload[]>(OSC_ROUTES_SNAPSHOT_COMMAND);
      this.routes = fresh;
    } catch (e) {
      this.routes = [];
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.isLoading = false;
    }
  }

  /** Bind `target` to the canonical route `route`. Refreshes the
   * snapshot on success so the consumer view re-renders with the
   * updated bind_target. On failure records the error and leaves
   * the snapshot untouched. */
  async bind(route: string, target: string): Promise<void> {
    this.error = null;
    try {
      await invoke(BIND_OSC_ROUTE_COMMAND, { route, target });
      await this.refresh();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  /** Clear the bind target on the canonical route `route`. Refreshes
   * the snapshot on success. */
  async unbind(route: string): Promise<void> {
    this.error = null;
    try {
      await invoke(UNBIND_OSC_ROUTE_COMMAND, { route });
      await this.refresh();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  /** Test-harness reset. Clears all state. */
  reset(): void {
    this.routes = [];
    this.isLoading = false;
    this.error = null;
  }
}

export const oscStore = new OscStore();
