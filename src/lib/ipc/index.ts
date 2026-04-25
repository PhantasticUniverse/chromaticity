// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Typed Tauri IPC wrappers for the Phase-1 D2 control round-trip.
// Frontend callers invoke `setControl` and `getControl`; the wrappers
// adapt the typed-binding payloads onto Tauri's `invoke()` contract
// and unwrap the engine-side `Result<T, String>`.

import { invoke } from '@tauri-apps/api/core';

import type { ControlId, ControlValue } from './bindings';

/// Set the control identified by `id` to `value`. Resolves once the
/// runtime has applied the committed event to canonical state.
export async function setControl(id: ControlId, value: ControlValue): Promise<void> {
  await invoke<void>('set_control', { id, value });
}

/// Read the current value of the control identified by `id`. Resolves
/// to `null` when the control has never been set.
export async function getControl(id: ControlId): Promise<ControlValue | null> {
  const result = await invoke<ControlValue | null>('get_control', { id });
  return result;
}
