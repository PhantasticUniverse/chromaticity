// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Collapsible layout-integrated Live Stage Monitor dock state store.
 *
 * Shared between `TopBar.svelte` (toggle button) and `AppShell.svelte`
 * (right-hand grid column on non-canvas views) so live stage preview never
 * floats over interactive controls.
 */

import { untrack } from 'svelte';

import { registerStateMachine } from '../state/diagnostics';

class StageDockStore {
  open = $state<boolean>(false);

  constructor() {
    registerStateMachine('stageDock', () =>
      untrack(() => ({
        open: this.open,
      })),
    );
  }

  toggle(): void {
    this.open = !this.open;
  }

  setOpen(next: boolean): void {
    this.open = next;
  }

  reset(): void {
    this.open = false;
  }
}

export const stageDockStore = new StageDockStore();
