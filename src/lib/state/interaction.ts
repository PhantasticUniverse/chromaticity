// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Pure discriminated-union interaction state machines for canvas/widget UX.
 */

/**
 * Pointer click-vs-drag discrimination machine (used by `Patch.svelte`).
 *
 * Transitions:
 *   idle ──pointerdown──▶ pending
 *   pending ──pointermove (< threshold)──▶ pending
 *   pending ──pointermove (≥ threshold)──▶ dragging (signals dragStart)
 *   pending ──pointerup──▶ idle (signals click)
 *   dragging ──pointerup──▶ idle (signals drop)
 */
export type DragOrClickState<Id extends string = string> =
  | { readonly kind: 'idle' }
  | {
      readonly kind: 'pending';
      readonly id: Id;
      readonly startX: number;
      readonly startY: number;
      readonly offsetX: number;
      readonly offsetY: number;
    }
  | {
      readonly kind: 'dragging';
      readonly id: Id;
      readonly offsetX: number;
      readonly offsetY: number;
    };

export const IDLE_DRAG_STATE: DragOrClickState<never> = { kind: 'idle' };

export function dragPointerDown<Id extends string>(
  id: Id,
  startX: number,
  startY: number,
  offsetX: number,
  offsetY: number,
): DragOrClickState<Id> {
  return { kind: 'pending', id, startX, startY, offsetX, offsetY };
}

export interface DragMoveResult<Id extends string> {
  readonly state: DragOrClickState<Id>;
  /** `true` on the exact transition from `'pending'` → `'dragging'`. */
  readonly promotedToDrag: boolean;
}

export function dragPointerMove<Id extends string>(
  state: DragOrClickState<Id>,
  x: number,
  y: number,
  thresholdPx: number,
): DragMoveResult<Id> {
  if (state.kind !== 'pending') {
    return { state, promotedToDrag: false };
  }
  const travel = Math.hypot(x - state.startX, y - state.startY);
  if (travel < thresholdPx) {
    return { state, promotedToDrag: false };
  }
  return {
    state: {
      kind: 'dragging',
      id: state.id,
      offsetX: state.offsetX,
      offsetY: state.offsetY,
    },
    promotedToDrag: true,
  };
}

export type DragUpOutcome<Id extends string> =
  | { readonly action: 'none'; readonly state: DragOrClickState<Id> }
  | { readonly action: 'click'; readonly id: Id; readonly state: DragOrClickState<Id> }
  | { readonly action: 'drop'; readonly id: Id; readonly state: DragOrClickState<Id> };

export function dragPointerUp<Id extends string>(state: DragOrClickState<Id>): DragUpOutcome<Id> {
  if (state.kind === 'idle') {
    return { action: 'none', state };
  }
  if (state.kind === 'pending') {
    return { action: 'click', id: state.id, state: { kind: 'idle' } };
  }
  return { action: 'drop', id: state.id, state: { kind: 'idle' } };
}

/**
 * Inline-rename state machine (used by `SnapBar.svelte`).
 *
 * Guarantees `draft` only exists while `kind === 'renaming'`.
 */
export type InlineRenameState<Slot = number> =
  | { readonly kind: 'idle' }
  | { readonly kind: 'renaming'; readonly slot: Slot; readonly draft: string };

export const IDLE_RENAME_STATE: InlineRenameState<never> = { kind: 'idle' };

export function beginRename<Slot>(slot: Slot, initialDraft: string): InlineRenameState<Slot> {
  return { kind: 'renaming', slot, draft: initialDraft };
}

export function updateRenameDraft<Slot>(
  state: InlineRenameState<Slot>,
  draft: string,
): InlineRenameState<Slot> {
  if (state.kind !== 'renaming') return state;
  return { kind: 'renaming', slot: state.slot, draft };
}

export function cancelRenameState<Slot>(): InlineRenameState<Slot> {
  return { kind: 'idle' };
}
