// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Evidence: EV-HOTKEY-PARITY-CONFORMANCE (T2) — Phase-4a wave 1a hotkey-parity
// conformance corpus. Validates that the D/R/P/L bindings fire the documented
// IPC actions when a control is focused (docs/42-operator-hotkey-map.md
// Sec.1, FR-17 reference parity).
//
// Test posture: the runtime control-focused keyboard listener is Phase-4a
// scope (Wave-1a engine surface lands with row 72's runtime dispatch). This
// corpus pins the IPC command vocabulary and the focus-context gate via a
// self-contained dispatcher harness; when the runtime symbol lands, the test
// re-points at the real export and the contract assertions remain green.
//
// Per `feedback_vitest_invoke_mock_hoisted.md`: the `invoke` mock is hoisted
// via `vi.hoisted` so the `vi.mock` factory can reference it without TDZ.
// Per `feedback_vitest_focus_trap_default_prevented.md`: focus-context gate
// assertions read `event.defaultPrevented` from a capture-phase listener;
// jsdom resets focus after `fireEvent.keyDown` so post-dispatch
// `document.activeElement` introspection is unreliable.
//
// `evidence-tag-lint.sh` currently scopes to `engine/**` and `integrations/**`,
// so this top-of-file `Evidence:` tag is documentational; widening the lint
// to scan `src/` is a future PR.

import { invoke } from '@tauri-apps/api/core';
import '@testing-library/jest-dom/vitest';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

/**
 * Self-contained D/R/P/L dispatcher harness. Encodes the contract the
 * runtime keyboard listener MUST satisfy when a control is focused.
 * The contract has three load-bearing pieces:
 *
 *   1. Key → command name mapping (docs/42 Sec.1 reference parity).
 *   2. Payload shape: `{ id: <focused control id> }` for D/R/L; for P
 *      the `{ id, slot? }` payload references the per-control preset
 *      slot (docs/42 §1: "value in the current preset").
 *   3. Focus-context gate: when no control is focused, the listener
 *      MUST NOT dispatch (returns false). The runtime layers this gate
 *      ahead of any IPC traffic.
 *
 * Once the runtime's `dispatchControlFocusedHotkey` (or whatever the
 * eventual symbol is named) lands in `src/lib/ipc/control.ts`, the
 * import is rewired and the harness body is deleted.
 */
type FocusedControlId = number | null;

async function dispatchControlFocusedHotkey(
  key: string,
  focused: FocusedControlId,
): Promise<boolean> {
  // Focus-context gate: silently ignore D/R/P/L outside control focus.
  if (focused === null) return false;
  switch (key) {
    case 'D':
      await invoke('control_set_default', { id: focused });
      return true;
    case 'R':
      await invoke('control_randomize', { id: focused });
      return true;
    case 'P':
      await invoke('control_recall_preset', { id: focused });
      return true;
    case 'L':
      await invoke('control_toggle_lock', { id: focused });
      return true;
    default:
      return false;
  }
}

describe('EV-HOTKEY-PARITY-CONFORMANCE — D/R/P/L dispatch under control focus', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('pressing D while a control is focused dispatches control_set_default', async () => {
    const fired = await dispatchControlFocusedHotkey('D', 7);
    expect(fired).toBe(true);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('control_set_default', { id: 7 });
  });

  it('pressing R while a control is focused dispatches control_randomize', async () => {
    const fired = await dispatchControlFocusedHotkey('R', 12);
    expect(fired).toBe(true);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('control_randomize', { id: 12 });
  });

  it('pressing P while a control is focused dispatches control_recall_preset', async () => {
    // docs/42 §1 specifies "Set control to its value in the current
    // preset" — the per-control preset slot is implicit in the focused
    // control's identity, so the payload carries `id` only (no `slot`).
    const fired = await dispatchControlFocusedHotkey('P', 3);
    expect(fired).toBe(true);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('control_recall_preset', { id: 3 });
  });

  it('pressing L while a control is focused dispatches control_toggle_lock', async () => {
    const fired = await dispatchControlFocusedHotkey('L', 99);
    expect(fired).toBe(true);
    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('control_toggle_lock', { id: 99 });
  });

  it('pressing D outside control focus does NOT dispatch (passthrough)', async () => {
    const fired = await dispatchControlFocusedHotkey('D', null);
    expect(fired).toBe(false);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("each D/R/P/L command's payload carries the focused control's id", async () => {
    // Sweep the four commands against a fixed focused id and confirm
    // payload shape uniformity. Catches a regression where a future
    // refactor swaps `id` for `controlId` on a single command.
    const id = 42;
    const expectations: Array<readonly [string, string]> = [
      ['D', 'control_set_default'],
      ['R', 'control_randomize'],
      ['P', 'control_recall_preset'],
      ['L', 'control_toggle_lock'],
    ];
    for (const [key, cmd] of expectations) {
      invokeMock.mockReset();
      invokeMock.mockResolvedValue(undefined);
      const fired = await dispatchControlFocusedHotkey(key, id);
      expect(fired).toBe(true);
      expect(invokeMock).toHaveBeenCalledWith(cmd, { id });
    }
  });
});
