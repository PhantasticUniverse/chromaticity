/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */
//
// Typed Tauri IPC wrappers for the Wave 2.1 Lane D.3 OSC route
// binding handlers (`osc_routes_snapshot` / `bind_osc_route` /
// `unbind_osc_route`).
//
// The route registry is process-local on the shell side (see the
// `osc_routes` field-level prose in `app/shell/src/state/mod.rs`);
// bind / unbind survives the process lifetime only, not across
// restarts. The OSC IP/port + TX/RX toggles persist via the existing
// `settingsStore` (Lane C.3 settings persistence).
//
// Lane D.3.

import { invoke } from '@tauri-apps/api/core';

import type { OscRoutePayload } from './bindings/OscRoutePayload';

const OSC_ROUTES_SNAPSHOT_COMMAND = 'osc_routes_snapshot';
const BIND_OSC_ROUTE_COMMAND = 'bind_osc_route';
const UNBIND_OSC_ROUTE_COMMAND = 'unbind_osc_route';

/**
 * Enumerate the operator-facing OSC route registry. The shell
 * pre-seeds the registry on bootstrap with the canonical UX subset
 * (matching `src/lib/data/osc-routes.ts`); operator bind / unbind
 * mutations are reflected in the snapshot.
 */
export async function oscRoutesSnapshot(): Promise<OscRoutePayload[]> {
  return invoke<OscRoutePayload[]>(OSC_ROUTES_SNAPSHOT_COMMAND);
}

/**
 * Bind `target` to the canonical route identified by `route`. The
 * shell-side handler rejects empty / whitespace-only targets and
 * unknown canonical addresses with an `Err(String)`.
 */
export async function bindOscRoute(route: string, target: string): Promise<void> {
  await invoke(BIND_OSC_ROUTE_COMMAND, { route, target });
}

/**
 * Clear the bind target on the canonical route identified by `route`.
 * Idempotent — unbinding an already-unbound route succeeds. Only
 * unknown canonical addresses fail.
 */
export async function unbindOscRoute(route: string): Promise<void> {
  await invoke(UNBIND_OSC_ROUTE_COMMAND, { route });
}
