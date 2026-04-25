// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Typed Tauri IPC wrappers for the Phase-1 D3 Snap round-trip.
// Frontend callers invoke `recallSnap` / `saveSnap` / `clearSnap` to
// dispatch through the runtime command bus, and read state back via
// `activeSnapSlot` / `snapSlotSaved`. Slot validation lives on the
// engine side (`SnapSlotId::new` rejects values above 15); the
// wrappers carry the raw `u8` across the boundary and surface the
// engine's failure string verbatim through the rejected promise.

import { invoke } from '@tauri-apps/api/core';

/// Recall the Snap slot identified by `slot`. Resolves once the
/// runtime has applied the committed event to canonical state.
export async function recallSnap(slot: number): Promise<void> {
  await invoke<void>('recall_snap', { slot });
}

/// Save the current state into the Snap slot identified by `slot`.
export async function saveSnap(slot: number): Promise<void> {
  await invoke<void>('save_snap', { slot });
}

/// Clear the Snap slot identified by `slot`.
export async function clearSnap(slot: number): Promise<void> {
  await invoke<void>('clear_snap', { slot });
}

/// Read the active Snap slot, or `null` when none is active.
export async function activeSnapSlot(): Promise<number | null> {
  return await invoke<number | null>('active_snap_slot');
}

/// Read whether the Snap slot identified by `slot` currently holds
/// saved state.
export async function snapSlotSaved(slot: number): Promise<boolean> {
  return await invoke<boolean>('snap_slot_saved', { slot });
}
