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

import type { RouteName } from './route.svelte';
import { registerStateMachine } from '../state/diagnostics';

const STAGE_DOCK_EXCLUDED_ROUTES: ReadonlySet<RouteName> = new Set(['perform', 'welcome']);

class StageDockStore {
  open = $state<boolean>(false);

  constructor() {
    registerStateMachine('stageDock', () =>
      untrack(() => ({
        open: this.open,
      })),
    );
  }

  isEligibleRoute(route: RouteName): boolean {
    return !STAGE_DOCK_EXCLUDED_ROUTES.has(route);
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
