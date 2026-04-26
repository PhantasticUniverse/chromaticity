/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
//
// Typed Tauri IPC wrapper for the W21 PR1 folder-picker stub.
//
// W21 PR1 ships the contract shape — `pick_folder(title) → Promise<string | null>`
// (null = user cancelled, string = absolute folder path). The backend
// returns a deterministic placeholder until the Phase-3+/4a
// `EV-INTEGRATION-MATRIX` slice swaps in `tauri-plugin-dialog`'s
// native folder picker. Settings → Storage section's "Browse" buttons
// invoke this wrapper to populate Media / Custom-scene / Backup
// folder rows.

import { invoke } from '@tauri-apps/api/core';

/// Open a native folder picker titled `title`. Resolves to the
/// absolute folder path on accept, or `null` when the user dismisses
/// the dialog. The W21 PR1 backend returns a deterministic
/// placeholder; native dialog wiring lands in Phase-3+/4a.
export async function pickFolder(title: string): Promise<string | null> {
  const result = await invoke<string | null>('pick_folder', { title });
  return result;
}
