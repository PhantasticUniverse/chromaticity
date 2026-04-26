// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// Wave-12 PR3 app-shell route store. In-memory `$state`-backed singleton
// keyed on a TypeScript string-literal union. NO URL routing — deep links
// and history sync are W13+ if signal warrants. The class instance pattern
// is the Svelte 5 idiomatic approach for shared module-level reactive
// state: `$state` runes inside a class field auto-track on read, and
// importing components react to `routeStore.current` changes without any
// subscription glue.
//
// `.svelte.ts` extension lets the Svelte compiler process `$state` outside
// a `.svelte` component. Plain `.ts` (like `library.ts`) cannot host runes
// — the W11 cache module sidesteps that by holding a non-reactive `Map`.
//
// Route names are explicitly enumerated rather than open-string so the
// SidebarRail's nav button list and AppShell's view dispatch share a
// single source of truth via exhaustiveness checks.

export type RouteName = 'perform' | 'library' | 'author' | 'settings';

class RouteStore {
  current: RouteName = $state('perform');

  set(name: RouteName): void {
    this.current = name;
  }
}

export const routeStore = new RouteStore();
