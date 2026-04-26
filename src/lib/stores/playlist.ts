/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
//
// Playlist round-trip helpers for the Phase-1 D4 IPC slice.
//
// Plain `.ts` (not `.svelte.ts`) so eslint-plugin-svelte's parser does
// not claim the file. Reactive state lives in the consuming component
// via `$state`; this module exposes the IPC sequencing.

import { advancePlaylist as advancePlaylistIpc, playlistPosition } from '../ipc/playlist';

/// Dispatch an `advance_playlist` for `step` and read the post-commit
/// cursor position back. Returns the new position.
export async function dispatchPlaylistAdvanceAndRead(step: number): Promise<number> {
  await advancePlaylistIpc(step);
  return await playlistPosition();
}

/// Read the current playlist cursor position without dispatching a
/// command. Used for mount-time hydration of the UI's playlist state.
export async function readPlaylistPosition(): Promise<number> {
  return await playlistPosition();
}
