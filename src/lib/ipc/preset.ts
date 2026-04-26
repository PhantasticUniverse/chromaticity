/* SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary */
/* SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> */
//
// Typed Tauri IPC wrappers for the Phase-1 D4 preset round-trip.
// Frontend callers invoke `recallPreset` to dispatch through the
// runtime command bus and read state back via `activePreset`. The
// engine-side `PresetId` is a transparent `u32` newtype with no
// validator (mirrors `ControlId`); the wrappers carry the raw `u32`
// across the boundary.

import { invoke } from '@tauri-apps/api/core';

import type { PresetId } from './bindings';

/// Recall the preset identified by `id`. Resolves once the runtime has
/// applied the committed event to canonical state.
export async function recallPreset(id: PresetId): Promise<void> {
  await invoke<void>('recall_preset', { id });
}

/// Read the active preset id, or `null` when no preset has been
/// recalled.
export async function activePreset(): Promise<PresetId | null> {
  return await invoke<PresetId | null>('active_preset');
}
