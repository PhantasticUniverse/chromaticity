/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
//
// Typed Tauri IPC wrappers for the Phase-1 D4 playlist round-trip.
// Frontend callers invoke `advancePlaylist` to step the runtime
// playlist cursor and read it back via `playlistPosition`. The
// engine-side cursor is an `i64` saturating at `i64::MAX` / `i64::MIN`
// per `engine/runtime/src/state.rs` apply-event semantics.

import { invoke } from '@tauri-apps/api/core';

/// Advance the active playlist by `step` entries (negative values move
/// backward). Resolves once the runtime has applied the committed
/// event to canonical state.
export async function advancePlaylist(step: number): Promise<void> {
  await invoke<void>('advance_playlist', { step });
}

/// Read the current playlist cursor position. Defaults to 0 before any
/// advance has been applied.
export async function playlistPosition(): Promise<number> {
  return await invoke<number>('playlist_position');
}
