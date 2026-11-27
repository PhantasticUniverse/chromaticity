/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Chromaticity */
//
// Typed Tauri IPC wrapper for the native folder-picker dialog.
//
// W21 PR1 shipped the contract shape against a deterministic backend
// stub (`/Users/operator/Movies/Chromaticity/Media`). Wave 2.7 SET.B
// swaps the stub for `@tauri-apps/plugin-dialog`'s native folder
// picker on macOS / Windows / Linux. The Rust backend in
// `app/shell/src/commands/picker.rs` keeps the legacy `pick_folder`
// Tauri command alive as a non-plugin fallback (and as the test-only
// invariant pinned in the existing backend unit tests).
//
// Resolution order:
//   1. Try `@tauri-apps/plugin-dialog` `open({ directory: true, title })`
//      — the production path. Loaded via dynamic import so the
//      vitest jsdom harness without the plugin still works.
//   2. Fall back to the legacy `invoke('pick_folder')` stub for any
//      environment where the plugin isn't present (vite-only dev,
//      pre-2.7 builds, etc.). Returns the stub placeholder.
//   3. Return `null` if both paths throw — the Storage section's
//      "Add Folder" button is a no-op when no dialog is available
//      rather than crashing.

import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

/// Open a native folder picker titled `title`. Resolves to the
/// absolute folder path on accept, or `null` when the user dismisses
/// the dialog. Wave 2.7 SET.B replaces the W21 PR1 stub with the
/// real `tauri-plugin-dialog` native dialog; the legacy `pick_folder`
/// IPC command remains as a non-plugin fallback for vite-only /
/// pre-plugin environments.
export async function pickFolder(title: string): Promise<string | null> {
  // Path 1: native `tauri-plugin-dialog` open(). The static import
  // above resolves at module-load; in environments without the
  // plugin runtime (vite-only dev / jsdom), the call itself
  // rejects and we fall through to the legacy IPC stub.
  try {
    const result = await openDialog({ directory: true, multiple: false, title });
    if (typeof result === 'string') {
      return result;
    }
    if (result === null) {
      return null;
    }
    // Array result (multiple-selection path) — not requested at
    // this seam, but defensively pick the first entry if the plugin
    // returns one anyway. Otherwise fall through to the IPC fallback.
    if (Array.isArray(result) && typeof result[0] === 'string') {
      return result[0];
    }
  } catch {
    // Plugin unavailable (no Tauri runtime, missing capability, etc.) —
    // fall through to the legacy IPC stub.
  }

  // Path 2: legacy `pick_folder` Tauri command. Returns the W21 PR1
  // deterministic placeholder under the existing Rust stub; future
  // backend swaps could surface a platform-specific dialog without
  // breaking the frontend contract.
  try {
    const result = await invoke<string | null>('pick_folder', { title });
    return result;
  } catch {
    // Path 3: no Tauri runtime at all (pure vite dev / jsdom unit
    // tests without an invoke mock). Return null so the calling
    // Storage button is a no-op rather than crashing.
    return null;
  }
}
