/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
//
// Typed Tauri IPC wrappers for the Wave 2.1 Lane C.3 operator
// settings persistence handlers (`get_settings` / `set_settings`).
//
// `get_settings()` reads the current persisted document from the
// shell-side `AppState.settings` mutex (loaded from disk on app
// bootstrap or falling back to default). `set_settings(payload)`
// replaces the in-memory document AND writes the on-disk TOML file
// at `~/Library/Application Support/<bundle id>/settings.toml`
// (macOS) / per-platform equivalent.
//
// Plan: `/Users/xaviermac/.claude/plans/inherited-percolating-quill.md`
// Lane C.3.

import { invoke } from '@tauri-apps/api/core';

import type { SettingsPayload } from './bindings/SettingsPayload';

/** Tauri command names surfaced by `app/shell/src/commands/settings.rs`. */
const GET_SETTINGS_COMMAND = 'get_settings';
const SET_SETTINGS_COMMAND = 'set_settings';

/**
 * Read the persisted operator-settings document from the shell. The
 * shell pre-loads the document on app bootstrap, so this resolves
 * synchronously against the in-memory mutex; the engine never blocks
 * on disk I/O at this seam.
 *
 * Throws when running outside the Tauri runtime (e.g. Vitest jsdom
 * with no mocked `invoke`). The `settingsStore` wrapper swallows the
 * failure and falls back to the bundle-faithful defaults so
 * `npm run dev` (vite-only) + browser-launched test harnesses keep
 * working.
 */
export async function getSettings(): Promise<SettingsPayload> {
  return invoke<SettingsPayload>(GET_SETTINGS_COMMAND);
}

/**
 * Replace the persisted operator-settings document. The shell
 * applies the in-memory swap + writes through to the on-disk TOML
 * file before resolving the promise; on resolve the on-disk state
 * matches the supplied payload.
 *
 * Throws when running outside the Tauri runtime (same fall-back
 * posture as `getSettings`).
 */
export async function setSettings(settings: SettingsPayload): Promise<void> {
  await invoke(SET_SETTINGS_COMMAND, { settings });
}
