/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
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
//      — the production path. A successful call resolves the pick (or
//      `null` on user dismiss) and never falls through to the stub.
//   2. If `open()` THROWS inside a real Tauri shell
//      (`window.__TAURI_INTERNALS__` present), the failure is a genuine
//      runtime fault (capability regression, plugin fault) — rethrow it
//      so the caller can surface the error. Falling through to the
//      legacy stub here would fabricate the deterministic placeholder
//      path and persist it as if the operator had picked it.
//   3. Outside a Tauri runtime (vite-only dev, jsdom unit tests), fall
//      back to the legacy `invoke('pick_folder')` stub; if that also
//      throws (no invoke mock at all), resolve `null` so the Storage
//      "Add Folder" button is a no-op rather than crashing.

import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

/** Same browser-mode signal `toIpcError` keys off in
 * `types/ipc-error.ts` and `views/Library.svelte` uses for its
 * marketplace CTA gate. Evaluated per call so test harnesses can
 * inject / remove the marker between cases. */
function tauriRuntimeAvailable(): boolean {
  return (
    typeof window !== 'undefined' &&
    Object.prototype.hasOwnProperty.call(window, '__TAURI_INTERNALS__')
  );
}

/// Open a native folder picker titled `title`. Resolves to the
/// absolute folder path on accept, or `null` when the user dismisses
/// the dialog. THROWS when the native dialog fails inside a real
/// Tauri shell — callers surface the failure via the `toIpcError` /
/// `<ErrorMessage>` idiom instead of receiving fabricated success
/// data. The legacy `pick_folder` IPC stub remains as a non-plugin
/// fallback for vite-only / jsdom environments only.
export async function pickFolder(title: string): Promise<string | null> {
  try {
    const result = await openDialog({ directory: true, multiple: false, title });
    if (typeof result === 'string') {
      return result;
    }
    // Array result (multiple-selection path) — not requested at this
    // seam, but defensively pick the first entry if the plugin returns
    // one anyway.
    if (Array.isArray(result) && typeof result[0] === 'string') {
      return result[0];
    }
    // `null` (user dismissed) or an empty / non-string array — no pick.
    return null;
  } catch (e) {
    if (tauriRuntimeAvailable()) {
      // Real shell: a plugin failure is a genuine runtime fault.
      // Surface it — never fabricate a placeholder path via the stub.
      throw e;
    }
    // No Tauri runtime injected — fall through to the legacy IPC stub.
  }

  try {
    const result = await invoke<string | null>('pick_folder', { title });
    return result;
  } catch {
    // No invoke mock either (pure vite dev / jsdom without a mock).
    return null;
  }
}
