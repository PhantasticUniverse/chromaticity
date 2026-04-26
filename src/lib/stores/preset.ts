/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
//
// Preset round-trip helpers for the Phase-1 D4 IPC slice.
//
// Plain `.ts` (not `.svelte.ts`) so eslint-plugin-svelte's parser does
// not claim the file. Reactive state lives in the consuming component
// via `$state`; this module exposes the IPC sequencing.

import type { PresetId } from '../ipc/bindings';
import { activePreset, recallPreset as recallPresetIpc } from '../ipc/preset';

/// Dispatch a `recall_preset` for `id` and read the post-commit active
/// preset back. Returns the read-back value (or `null` when the
/// runtime reports no active preset, which should not happen on the
/// happy path).
export async function dispatchPresetRecallAndRead(id: PresetId): Promise<PresetId | null> {
  await recallPresetIpc(id);
  return await activePreset();
}

/// Read the current active preset without dispatching a command. Used
/// for mount-time hydration of the UI's preset state.
export async function readActivePreset(): Promise<PresetId | null> {
  return await activePreset();
}
