// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { beforeEach, describe, expect, test } from 'vitest';

import {
  beginRename,
  beginRequest,
  cancelRenameState,
  dragPointerDown,
  dragPointerMove,
  dragPointerUp,
  failRequest,
  IDLE_DRAG_STATE,
  IDLE_RENAME_STATE,
  idleResource,
  inspectAllStateMachines,
  isResourceLoading,
  overlayStack,
  registerStateMachine,
  resourceError,
  succeedRequest,
  updateRenameDraft,
} from './index';

describe('AsyncResource state machine', () => {
  test('transitions idle -> loading -> ready and exposes compat projections', () => {
    const r0 = idleResource<string[]>([]);
    expect(r0.kind).toBe('idle');
    expect(isResourceLoading(r0)).toBe(false);
    expect(resourceError(r0)).toBeNull();

    const r1 = beginRequest(r0, 1);
    expect(r1.kind).toBe('loading');
    expect(isResourceLoading(r1)).toBe(true);
    expect(resourceError(r1)).toBeNull();

    const r2 = succeedRequest(r1, 1, ['a', 'b']);
    expect(r2.kind).toBe('ready');
    expect(r2.data).toEqual(['a', 'b']);
    expect(isResourceLoading(r2)).toBe(false);
    expect(resourceError(r2)).toBeNull();
  });

  test('drops stale out-of-order responses and stale errors', () => {
    const r1 = beginRequest(idleResource<string | null>(null), 1);
    const r2 = beginRequest(r1, 2);

    // Request 1 resolves after request 2 started — must be ignored.
    const afterStaleOk = succeedRequest(r2, 1, 'stale-scene');
    expect(afterStaleOk).toBe(r2);

    // Request 1 fails after request 2 started — must be ignored.
    const afterStaleErr = failRequest(r2, 1, 'stale-error');
    expect(afterStaleErr).toBe(r2);

    // Request 2 resolves — accepted.
    const final = succeedRequest(r2, 2, 'fresh-scene');
    expect(final.kind).toBe('ready');
    expect(final.data).toBe('fresh-scene');
  });

  test('failRequest preserves or resets data according to fallbackData', () => {
    const loaded = succeedRequest(beginRequest(idleResource<string[]>([]), 1), 1, ['x']);
    const reloading = beginRequest(loaded, 2);

    const kept = failRequest(reloading, 2, 'boom');
    expect(kept.kind).toBe('error');
    expect(kept.data).toEqual(['x']);
    expect(resourceError(kept)).toBe('boom');

    const cleared = failRequest(reloading, 2, 'boom', []);
    expect(cleared.kind).toBe('error');
    expect(cleared.data).toEqual([]);
  });
});

describe('OverlayStack', () => {
  beforeEach(() => {
    overlayStack.reset();
  });

  test('manages LIFO Esc precedence and idempotent open/close/toggle', () => {
    expect(overlayStack.top).toBeNull();

    overlayStack.open('console');
    overlayStack.open('command-palette');
    expect(overlayStack.top).toBe('command-palette');
    expect(overlayStack.isOpen('console')).toBe(true);

    // Re-opening console moves it to top without duplicating.
    overlayStack.open('console');
    expect(overlayStack.stack).toEqual(['command-palette', 'console']);

    expect(overlayStack.closeTop()).toBe('console');
    expect(overlayStack.top).toBe('command-palette');

    overlayStack.toggle('command-palette');
    expect(overlayStack.top).toBeNull();
    expect(overlayStack.closeTop()).toBeNull();
  });
});

describe('Interaction reducers', () => {
  test('DragOrClickState distinguishes click (< threshold) from drag (>= threshold)', () => {
    expect(IDLE_DRAG_STATE.kind).toBe('idle');

    // Click path
    const p1 = dragPointerDown('node-a', 10, 10, 2, 2);
    const m1 = dragPointerMove(p1, 12, 11, 4);
    expect(m1.promotedToDrag).toBe(false);
    expect(m1.state.kind).toBe('pending');
    const upClick = dragPointerUp(m1.state);
    expect(upClick.action).toBe('click');
    if (upClick.action === 'click') {
      expect(upClick.id).toBe('node-a');
    }
    expect(upClick.state.kind).toBe('idle');

    // Drag path
    const p2 = dragPointerDown('node-b', 10, 10, 3, 4);
    const m2 = dragPointerMove(p2, 20, 10, 4);
    expect(m2.promotedToDrag).toBe(true);
    expect(m2.state.kind).toBe('dragging');
    const upDrop = dragPointerUp(m2.state);
    expect(upDrop.action).toBe('drop');
    if (upDrop.action === 'drop') {
      expect(upDrop.id).toBe('node-b');
    }
    expect(upDrop.state.kind).toBe('idle');
  });

  test('InlineRenameState transitions idle <-> renaming cleanly', () => {
    expect(IDLE_RENAME_STATE.kind).toBe('idle');
    const r1 = beginRename(3, 'Warm');
    expect(r1).toEqual({ kind: 'renaming', slot: 3, draft: 'Warm' });
    const r2 = updateRenameDraft(r1, 'Cool');
    expect(r2).toEqual({ kind: 'renaming', slot: 3, draft: 'Cool' });
    expect(cancelRenameState<number>().kind).toBe('idle');
  });
});

describe('State-machine diagnostics registry', () => {
  test('inspectAllStateMachines returns deterministic sorted snapshots', () => {
    registerStateMachine('test-machine-z', () => ({ kind: 'idle' }));
    const snap = inspectAllStateMachines();
    expect(snap['overlayStack']).toBeDefined();
    expect(snap['test-machine-z']).toEqual({ kind: 'idle' });
  });
});
