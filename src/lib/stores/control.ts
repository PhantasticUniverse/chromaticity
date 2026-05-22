// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Control round-trip helper for the Phase-1 D2 IPC slice.
//
// Plain `.ts` (not `.svelte.ts`) so eslint-plugin-svelte's parser does
// not claim the file. Reactive state lives in the consuming component
// via `$state`; this module exposes the IPC sequencing.

import { invoke } from '@tauri-apps/api/core';

import { getControl, setControl } from '../ipc';
import type { ControlId, ControlValue, ControlValue2 } from '../ipc/bindings';

/// Dispatch `value` to the runtime via `set_control`, then read the
/// committed value back via `get_control`. Returns the read-back value
/// (or `null` if the runtime reports the control is unset, which
/// should not happen on the happy path).
export async function dispatchAndRead(
  id: ControlId,
  value: ControlValue,
): Promise<ControlValue | null> {
  await setControl(id, value);
  return await getControl(id);
}

/// Dispatch a 2D (XY) value to the runtime via `set_control_2d`, then
/// read the committed pair back via `get_control_2d`. Returns the
/// read-back pair (or `null` if the runtime reports the control is
/// unset). Mirrors `dispatchAndRead` for the scalar path.
///
/// Callers should check `controlLockStore.isLocked(id)` before
/// calling this function — the Perform view's `dispatchXy` wrapper
/// performs that check.
export async function dispatchXy(
  id: ControlId,
  value: ControlValue2,
): Promise<ControlValue2 | null> {
  await invoke<void>('set_control_2d', { id, value });
  const result = await invoke<ControlValue2 | null>('get_control_2d', { id });
  return result;
}
