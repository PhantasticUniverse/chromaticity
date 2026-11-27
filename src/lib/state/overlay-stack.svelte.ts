// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { registerStateMachine } from './diagnostics';

/**
 * Canonical identifiers for modal / drawer overlays managed by `overlayStack`.
 * Ordering in the stack determines `Esc` close precedence (topmost closes first).
 */
export type OverlayId =
  | 'hotkeys'
  | 'command-palette'
  | 'console'
  | 'author-backup'
  | 'author-compare';

/**
 * Single explicit overlay stack replacing scattered `open` booleans across
 * `App.svelte`, `AppShell.svelte`, `consoleStore`, and view modals.
 */
class OverlayStack {
  /** Ordered open overlay stack (last element is topmost). */
  stack: readonly OverlayId[] = $state([]);

  /** Topmost open overlay, or `null` when none are open. */
  get top(): OverlayId | null {
    return this.stack.length > 0 ? (this.stack[this.stack.length - 1] ?? null) : null;
  }

  /** Whether `id` is currently open anywhere in the stack. */
  isOpen(id: OverlayId): boolean {
    return this.stack.includes(id);
  }

  /** Open `id` and move it to the top of the stack (idempotent). */
  open(id: OverlayId): void {
    this.stack = [...this.stack.filter((entry) => entry !== id), id];
  }

  /** Close `id` if present (idempotent). */
  close(id: OverlayId): void {
    if (!this.stack.includes(id)) return;
    this.stack = this.stack.filter((entry) => entry !== id);
  }

  /** Toggle `id` open/closed. */
  toggle(id: OverlayId): void {
    if (this.isOpen(id)) {
      this.close(id);
    } else {
      this.open(id);
    }
  }

  /** Close the topmost overlay (standard `Esc` handler). Returns closed id or `null`. */
  closeTop(): OverlayId | null {
    const currentTop = this.top;
    if (currentTop === null) return null;
    this.stack = this.stack.slice(0, -1);
    return currentTop;
  }

  /** Close all overlays (route reset / test harness). */
  reset(): void {
    this.stack = [];
  }
}

export const overlayStack = new OverlayStack();

registerStateMachine('overlayStack', () => ({
  top: overlayStack.top,
  stack: [...overlayStack.stack],
}));
