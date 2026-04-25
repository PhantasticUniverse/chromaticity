// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Snap round-trip helpers for the Phase-1 D3 IPC slice.
//
// Plain `.ts` (not `.svelte.ts`) so eslint-plugin-svelte's parser does
// not claim the file. Reactive state lives in the consuming component
// via `$state`; this module exposes the IPC sequencing.

import {
  activeSnapSlot,
  clearSnap as clearSnapIpc,
  recallSnap as recallSnapIpc,
  saveSnap as saveSnapIpc,
  snapSlotSaved,
} from '../ipc/snap';

/// Sixteen Snap slots — `SnapSlotId::MAX` on the engine side is 15.
const SLOT_COUNT = 16;

/// Aggregated snap-state read-back: which slot (if any) is active and
/// the saved-bitmask projected as a boolean array indexed by slot.
export interface SnapState {
  active: number | null;
  saved: boolean[];
}

/// Dispatch a `save_snap` for `slot` and resolve once the runtime has
/// applied the committed event.
export async function saveSnap(slot: number): Promise<void> {
  await saveSnapIpc(slot);
}

/// Dispatch a `recall_snap` for `slot` and resolve once the runtime
/// has applied the committed event.
export async function recallSnap(slot: number): Promise<void> {
  await recallSnapIpc(slot);
}

/// Dispatch a `clear_snap` for `slot` and resolve once the runtime
/// has applied the committed event.
export async function clearSnap(slot: number): Promise<void> {
  await clearSnapIpc(slot);
}

/// Walk slots `0..SLOT_COUNT` and aggregate `snap_slot_saved` plus the
/// active slot id into a single snapshot. Mirrors D2's
/// `dispatchAndRead` shape: callers chain a mutator with this reader
/// to render canonical state after each round-trip.
export async function readSnapState(): Promise<SnapState> {
  const active = await activeSnapSlot();
  const savedQueries: Promise<boolean>[] = [];
  for (let slot = 0; slot < SLOT_COUNT; slot += 1) {
    savedQueries.push(snapSlotSaved(slot));
  }
  const saved = await Promise.all(savedQueries);
  return { active, saved };
}
